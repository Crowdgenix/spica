#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![feature(min_specialization)]

pub mod traits;
pub use traits::{*};

#[ink::contract]
pub mod ido {
    use hex::*;
    use ink::{
        env::{
            hash,
            DefaultEnvironment,
        },
        codegen::{
            EmitEvent,
            Env,
        },
        prelude::string::{String, ToString},
        prelude::vec::Vec,
        reflect::ContractEventBase,
        storage::{
            Mapping,
        },
        ToAccountId,
    };
    use crate::{traits};
    use crate::traits::{IDOError};
    use token::token::TokenRef;
    use logics::traits::token::PSP22;
    use logics::{ensure};
    use logics::traits::common::ZERO_ADDRESS;

    type Event = <IdoContract as ContractEventBase>::Type;

    type RoleType = u32;
    pub const SUB_ADMIN: RoleType = ink::selector_id!("SUB_ADMIN");

    #[ink(event)]
    pub struct InitIdoContract {
        #[ink(topic)]
        pub ido_token: AccountId, // token for presale
        pub price: u128, // price of token, IDO amount = received_value * price / 10^price_decimals
        pub price_decimals: u32, // decimals of price
        pub signer: AccountId, // signer of smart contract, signer has the role to sign the message that approving the user to buy the IDO token
        pub max_issue_ido_amount: u128, // max amount of IDO that can be issued by the smart contract for each user
    }

    // we need to emit the event for the backend crawler can handle the buy token event of each user
    #[ink(event)]
    pub struct BuyTokenWithNative {
        #[ink(topic)]
        pub buyer: AccountId, // the buyer
        pub native_amount: u128, // the native amount to swap to IDO token
        pub ido_token_amount: u128, // the IDO token amount, calculated by native_amount * price / 10^price_decimals
        pub nonce: u128, // nonce to defend against attack (user can use the signature twice or more times if we don't have the nonce)
    }

    // we need to emit the event for the backend crawler can handle the claim token event of each user
    #[ink(event)]
    pub struct ClaimToken {
        #[ink(topic)]
        pub buyer: AccountId, // the claimer
        pub ido_token_amount: u128, // the claimed amount of IDO token
        pub nonce: u128, // nonce to defend against attack
    }

    #[ink(storage)]
    pub struct IdoContract {
        ido_token: TokenRef, // IDO token for presale
        price: u128, // price of token, IDO amount = received_value * price / 10^price_decimals
        price_decimals: u32, // decimals of price
        signer: AccountId, // signer of smart contract, signer has the role to sign the message that approving the user to buy the IDO token
        account_nonce: Mapping<AccountId, u128>, // nonce of each user to defend against attack
        user_ido_balances: Mapping<AccountId, u128>, // bought amounts balances of each user
        max_issue_ido_amount: u128, // max amount of IDO that can be issued by the smart contract for each user
        issued_ido_amount: u128, // total issued IDO amount
        roles: Mapping<(AccountId, RoleType), bool>, // roles mapping
        owner: AccountId, // owner of smart contract
    }

    impl IdoContract {
        /// constructor of IDO contract, we set _owner: AccountId, _ido_token: AccountId, _signer: AccountId, _price: u128, _price_decimals: u32, _max_issue_ido_amount: u128
        #[ink(constructor)]
        pub fn new(_owner: AccountId, _ido_token: AccountId, _signer: AccountId, _price: u128, _price_decimals: u32, _max_issue_ido_amount: u128) -> Self {
            let instance = Self {
                ido_token: ink::env::call::FromAccountId::from_account_id(_ido_token),
                price: _price,
                price_decimals: _price_decimals,
                signer: _signer,
                account_nonce: Mapping::default(),
                user_ido_balances: Mapping::default(),
                max_issue_ido_amount: _max_issue_ido_amount,
                issued_ido_amount: 0,
                owner: _owner,
                roles: Mapping::default(),
            };
            instance._emit_init_ido_contract_event(_ido_token, _price, _price_decimals, _signer, _max_issue_ido_amount);

            instance
        }

        /// function to get balance of ido token
        #[ink(message)]
        pub fn get_ido_token_balance(&self, account: AccountId) -> u128 {
            return self.user_ido_balances.get(&account).unwrap_or(0 as u128);
        }


        // get owner of the smart contract
        #[ink(message)]
        pub fn owner(&self) -> AccountId {
            self.owner.clone()
        }

        // function to update code_hash (logic of IDO contract)
        #[ink(message)]
        pub fn set_code(&mut self, code_hash: [u8; 32]) -> Result<(), IDOError> {
            ensure!(self.env().caller() == self.owner, IDOError::NotOwner);
            ink::env::set_code_hash(&code_hash).unwrap_or_else(|err| {
                panic!(
                    "Failed to `set_code_hash` to {:?} due to {:?}",
                    code_hash, err
                )
            });
            ink::env::debug_println!("Switched code hash to {:?}.", code_hash);
            Ok(())
        }

        // owner can transfer ownership of the smart contract to another account
        #[ink(message)]
        pub fn transfer_ownership(&mut self, new_owner: AccountId) -> Result<(), IDOError> {
            ensure!(self.env().caller() == self.owner, IDOError::NotOwner);
            self.owner = new_owner;
            Ok(())
        }

        // owner can grant role for user to be the sub admin
        #[ink(message)]
        pub fn grant_role(&mut self, role: RoleType, user: AccountId) -> Result<(), IDOError> {
            ensure!(self.env().caller() == self.owner, IDOError::NotOwner);
            self.roles.insert(&(user, role), &true);
            Ok(())
        }

        // owner can revoke role of user
        #[ink(message)]
        pub fn revoke_role(&mut self, role: RoleType, user: AccountId) -> Result<(), IDOError> {
            ensure!(self.env().caller() == self.owner, IDOError::NotOwner);
            self.roles.insert(&(user, role), &false);
            Ok(())
        }

        // check that is user has the given role or not
        #[ink(message)]
        pub fn is_role_granted(&self, role: RoleType, user: AccountId) -> Result<bool, IDOError> {
            let ok = self.roles.get(&(user, role)).unwrap_or(false);
            Ok(ok)
        }

        // this function is used to generate the message that will be signed by the signer, the message contains the buy_ido_ signature, ido token, caller, amount, deadline, nonce, this ensures the message is unique
        #[ink(message)]
        pub fn gen_msg_for_buy_token(&self, deadline: Timestamp, nonce: u128, received_value: u128) -> String {
            // generate message = buy_ido + ido_token + buyer + amount
            let mut message: String = String::from("");
            message.push_str("buy_ido_");
            message.push_str(encode(&self.ido_token.to_account_id()).as_str());
            message.push_str("_");
            message.push_str(encode(&self.env().caller()).as_str());
            message.push_str("_");
            message.push_str(&received_value.to_string().as_str());
            message.push_str("_");
            message.push_str(&deadline.to_string().as_str());
            message.push_str("_");
            message.push_str(&nonce.to_string().as_str());

            message
        }

        // this function is used to generate the message that will be signed by the signer, the message contains the buy_ido_ claim_ido_token_, ido token, caller, amount, deadline, nonce, this ensures the message is unique
        #[ink(message)]
        pub fn gen_msg_for_claim_token(&self, deadline: Timestamp, nonce: u128, amount: u128) -> String {
            let mut message: String = String::from("");
            message.push_str("claim_ido_token_");
            message.push_str(encode(&self.ido_token.to_account_id()).as_str());
            message.push_str("_");
            message.push_str(encode(&self.env().caller()).as_str());
            message.push_str("_");
            message.push_str(&amount.to_string().as_str());
            message.push_str("_");
            message.push_str(&deadline.to_string().as_str());
            message.push_str("_");
            message.push_str(&nonce.to_string().as_str());

            message
        }

        // get the signer of the smart contract
        #[ink(message)]
        pub fn get_signer(&self) -> AccountId {
            self.signer
        }

        // sub admin can set the signer of the smart contract
        #[ink(message)]
        pub fn set_signer(&mut self, _new_signer: AccountId) -> Result<(), IDOError> {
            let is_admin: bool = self.roles.get(&(self.env().caller(), SUB_ADMIN)).unwrap_or(false);
            ensure!(is_admin, IDOError::NotAdmin);
            self.signer = _new_signer;
            Ok(())
        }

        // this function use to verify the message and the signature, it returns false if the signature isn't signed by the signer
        #[ink(message)]
        pub fn verify_signature(&self, signature: [u8; 65], msg: String) -> bool {
            self._verify(msg, self.signer, signature)
        }

        /// get ido token
        #[ink(message)]
        pub fn get_ido_token(&self) -> AccountId {
            self.ido_token.to_account_id()
        }

        // get nonce of user
        #[ink(message)]
        pub fn get_nonce(&self, account: AccountId) -> u128 {
            self.account_nonce.get(&account).unwrap_or(0)
        }

        /// function to buy ido token with native
        #[ink(message, payable)]
        pub fn buy_ido_with_native(&mut self, deadline: Timestamp, nonce: u128, signature: [u8; 65]) -> Result<(), IDOError> {
            // we ensure the deadline and nonce are correct
            ensure!(
                deadline >= self.env().block_timestamp(),
                IDOError::Expired
            );
            ensure!(
                nonce == self.account_nonce.get(&self.env().caller()).unwrap_or(0),
                IDOError::InvalidNonce(nonce.to_string())
            );


            // increase nonce of user
            self.account_nonce.insert(&self.env().caller(), &(nonce + 1));

            let received_value = Self::env().transferred_value();

            // generate message = buy_ido + ido_token + buyer + amount
            let message = self.gen_msg_for_buy_token(deadline, nonce, received_value);

            // verify signature
            let is_ok = self._verify(message, self.signer, signature);

            ensure!(is_ok, IDOError::InvalidSignature);


            // calculate IDO amount = received_value * price / 10^price_decimals
            let ido_amount = received_value.checked_mul(self.price).unwrap().checked_div((10 as u128).checked_pow(self.price_decimals).unwrap()).unwrap();
            // ensure that the amount is less than the max_issue_ido_amount
            ensure!(
                self.issued_ido_amount.checked_add(ido_amount).unwrap() <= self.max_issue_ido_amount,
                IDOError::MaxIssueIdoAmount,
            );
            // update the issued_ido_amount
            self.issued_ido_amount = self.issued_ido_amount.checked_add(ido_amount).unwrap();

            let old_balances = match self.user_ido_balances.get(&self.env().caller()) {
                Some(balance) => balance,
                None => 0 as u128,
            };
            let new_balances = old_balances.checked_add(ido_amount).unwrap();
            // update the user_ido_balances
            self.user_ido_balances.insert(self.env().caller(), &new_balances);

            // emit event
            self._emit_buy_with_native_event(self.env().caller(), received_value, ido_amount, nonce);
            Ok(())
        }

        /// function to claim ido token
        #[ink(message)]
        pub fn claim_ido_token(&mut self, deadline: Timestamp, nonce: u128, amount: u128, signature: [u8; 65]) -> Result<(), IDOError> {
            // we ensure the deadline and nonce are correct
            ensure!(
                deadline >= self.env().block_timestamp(),
                IDOError::Expired
            );

            ensure!(
                nonce == self.account_nonce.get(&self.env().caller()).unwrap_or(0),
                IDOError::InvalidNonce(nonce.to_string())
            );

            // update nonce of user
            self.account_nonce.insert(&self.env().caller(), &(nonce + 1));

            let caller = self.env().caller();
            // ensure the user has enough collateral assets
            ensure!(self.ido_token.balance_of(self.env().account_id()) >= amount, IDOError::InsufficientBalance);

            // generate message
            let message = self.gen_msg_for_claim_token(deadline, nonce, amount);

            // verify signature
            let is_ok = self._verify(message, self.signer, signature);
            ensure!(is_ok, IDOError::InvalidSignature);

            let old_balances = self.user_ido_balances.get(&self.env().caller()).unwrap_or(0);
            let new_balances = old_balances.checked_sub(amount).unwrap_or(0);
            // update the user_ido_balances
            self.user_ido_balances.insert(self.env().caller(), &new_balances);

            let result = self.ido_token.transfer(caller, amount, Vec::new());
            // check result
            ensure!(!result.is_err(), IDOError::SafeTransferError);

            self._emit_claim_token_event(caller, amount, nonce);
            Ok(())
        }

        /// function to set price of ido token, only admin can call this function
        #[ink(message)]
        pub fn admin_set_price(&mut self, new_price: u128) -> Result<(), IDOError> {
            let is_admin: bool = self.roles.get(&(self.env().caller(), SUB_ADMIN)).unwrap_or(false);
            ensure!(is_admin, IDOError::NotAdmin);
            self.price = new_price;
            Ok(())
        }

        /// function to get price of ido token
        #[ink(message)]
        pub fn get_price(&self) -> u128 {
            self.price
        }

        fn _verify(&self, data: String, signer: AccountId, signature: [u8; 65]) -> bool {
            ink::env::debug_println!("data {:?}", data);
            ink::env::debug_println!("signer {:?}", signer);
            ink::env::debug_println!("signature {:?}", signature);

            let mut message_hash = <hash::Blake2x256 as hash::HashOutput>::Type::default();
            ink::env::hash_bytes::<hash::Blake2x256>(&data.as_bytes(), &mut message_hash);

            ink::env::debug_println!("message_hash {:?}", message_hash);

            let output = self.env().ecdsa_recover(&signature, &message_hash).expect("Failed to recover");

            ink::env::debug_println!("pubkey {:?}", output);

            let mut signature_account_id = <hash::Blake2x256 as hash::HashOutput>::Type::default();
            ink::env::hash_encoded::<hash::Blake2x256, _>(&output, &mut signature_account_id);

            ink::env::debug_println!("Sig account id {:?}", AccountId::from(signature_account_id));

            signer == AccountId::from(signature_account_id)
        }

        fn _emit_buy_with_native_event(&self, _buyer: AccountId, _native_amount: u128, _ido_token_amount: u128, _nonce: u128) {
            Self::emit_event(Self::env(), Event::BuyTokenWithNative(BuyTokenWithNative {
                buyer: _buyer,
                native_amount: _native_amount,
                ido_token_amount: _ido_token_amount,
                nonce: _nonce,
            }));
        }

        fn _emit_claim_token_event(&self, _buyer: AccountId, _ido_token_amount: u128, _nonce: u128) {
            Self::emit_event(Self::env(), Event::ClaimToken(ClaimToken {
                buyer: _buyer,
                ido_token_amount: _ido_token_amount,
                nonce: _nonce,
            }));
        }

        fn _emit_init_ido_contract_event(&self, _ido_token: AccountId, _price: u128, _price_decimals: u32, _signer: AccountId, _max_issue_ido_amount: u128) {
            Self::emit_event(Self::env(), Event::InitIdoContract(InitIdoContract {
                ido_token: _ido_token,
                price: _price,
                price_decimals: _price_decimals,
                signer: _signer,
                max_issue_ido_amount: _max_issue_ido_amount,
            }));
        }

        fn emit_event<EE>(emitter: EE, event: Event)
            where
                EE: EmitEvent<IdoContract>,
        {
            emitter.emit_event(event);
        }
    }
}
