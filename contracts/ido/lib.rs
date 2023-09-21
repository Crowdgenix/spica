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
    use crate::{ensure, traits};
    use crate::traits::{IDOError};
    use token::token::TokenRef;
    use logics::traits::token::PSP22;

    type Event = <IdoContract as ContractEventBase>::Type;

    pub const ZERO_ADDRESS: [u8; 32] = [255; 32];

    type RoleType = u32;
    pub const SUB_ADMIN: RoleType = ink::selector_id!("SUB_ADMIN");

    #[ink(event)]
    pub struct InitIdoContract {
        #[ink(topic)]
        pub ido_token: AccountId,
        pub price: u128,
        pub price_decimals: u32,
        pub signer: AccountId,
        pub max_issue_ido_amount: u128,
    }

    #[ink(event)]
    pub struct BuyTokenWithNative {
        #[ink(topic)]
        pub buyer: AccountId,
        pub native_amount: u128,
        pub ido_token_amount: u128,
        pub nonce: u128,
    }

    #[ink(event)]
    pub struct ClaimToken {
        #[ink(topic)]
        pub buyer: AccountId,
        pub ido_token_amount: u128,
        pub nonce: u128,
    }

    #[ink(storage)]
    pub struct IdoContract {
        ido_token: TokenRef,
        price: u128,
        price_decimals: u32,
        signer: Option<AccountId>,
        account_nonce: Mapping<AccountId, u128>,
        user_ido_balances: Mapping<AccountId, u128>,
        max_issue_ido_amount: u128,
        issued_ido_amount: u128,
        roles: Mapping<(AccountId, RoleType), bool>,
        owner: AccountId,
        is_initialized: bool,
    }

    impl IdoContract {
        /// constructor of IDO contract
        #[ink(constructor)]
        pub fn new(owner: AccountId) -> Self {
            Self {
                ido_token: ink::env::call::FromAccountId::from_account_id(ZERO_ADDRESS.into()),
                price: 0,
                price_decimals: 5,
                signer: Option::None,
                account_nonce: Mapping::default(),
                user_ido_balances: Mapping::new(),
                max_issue_ido_amount: 0,
                issued_ido_amount: 0,
                is_initialized: false,
                owner: Self::env().caller(),
                roles: Mapping::default(),
            }
        }

        /// function to get balance of ido token
        #[ink(message)]
        pub fn get_ido_token_balance(&self, account: AccountId) -> u128 {
            return self.user_ido_balances.get(&account).unwrap_or(0 as u128);
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

        #[ink(message)]
        pub fn transfer_ownership(&mut self, new_owner: AccountId) -> Result<(), IDOError> {
            ensure!(self.env().caller() == self.owner, IDOError::NotOwner);
            self.owner = new_owner;
            Ok(())
        }

        #[ink(message)]
        pub fn grant_role(&mut self, role: RoleType, user: AccountId) -> Result<(), IDOError> {
            ensure!(self.env().caller() == self.owner, IDOError::NotOwner);
            self.roles.insert(&(user, role), &true);
            Ok(())
        }

        #[ink(message)]
        pub fn revoke_role(&mut self, role: RoleType, user: AccountId) -> Result<(), IDOError> {
            ensure!(self.env().caller() == self.owner, IDOError::NotOwner);
            self.roles.insert(&(user, role), &false);
            Ok(())
        }

        #[ink(message)]
        pub fn is_role_granted(&self, role: RoleType, user: AccountId) -> Result<bool, IDOError> {
            let ok = self.roles.get(&(user, role)).unwrap_or(false);
            Ok(ok)
        }

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

        #[ink(message)]
        pub fn get_signer(&self) -> Option<AccountId> {
            self.signer
        }

        #[ink(message)]
        pub fn set_signer(&mut self, _new_signer: AccountId) -> Result<(), IDOError> {
            let is_admin: bool = self.roles.get(&(self.env().caller(), SUB_ADMIN)).unwrap_or(false);
            ensure!(is_admin, IDOError::NotAdmin);
            self.signer = Some(_new_signer);
            Ok(())
        }

        #[ink(message)]
        pub fn verify_signature(&self, signature: [u8; 65], msg: String) -> bool {
            self._verify(msg, self.signer.unwrap(), signature)
        }

        /// this function is initialised function, will init the contract properties
        #[ink(message)]
        pub fn init_ido(&mut self, _ido_token: AccountId, _signer: AccountId, _price: u128, _price_decimals: u32, _max_issue_ido_amount: u128) -> Result<(), IDOError> {
            ensure!(self.is_initialized == false, IDOError::Initialized);
            self.ido_token = ink::env::call::FromAccountId::from_account_id(_ido_token);
            self.price = _price;
            self.price_decimals = _price_decimals;
            self.signer = Some(_signer);
            self.max_issue_ido_amount = _max_issue_ido_amount;
            self.is_initialized = true;

            self._emit_init_ido_contract_event(_ido_token, _price, _price_decimals, _signer, _max_issue_ido_amount);
            Ok(())
        }

        /// get ido token
        #[ink(message)]
        pub fn get_ido_token(&self) -> AccountId {
            self.ido_token.to_account_id()
        }

        #[ink(message)]
        pub fn get_nonce(&self, account: AccountId) -> u128 {
            self.account_nonce.get(&account).unwrap_or(0)
        }

        /// function to buy ido token with native
        #[ink(message, payable)]
        pub fn buy_ido_with_native(&mut self, deadline: Timestamp, nonce: u128, signature: [u8; 65]) -> Result<(), IDOError> {
            ensure!(
                deadline >= self.env().block_timestamp(),
                IDOError::Expired
            );
            ensure!(
                nonce == self.account_nonce.get(&self.env().caller()).unwrap_or(0),
                IDOError::InvalidNonce(nonce.to_string())
            );


            self.account_nonce.insert(&self.env().caller(), &(nonce + 1));

            let received_value = Self::env().transferred_value();

            // generate message = buy_ido + ido_token + buyer + amount
            let message = self.gen_msg_for_buy_token(deadline, nonce, received_value);

            // verify signature
            let is_ok = self._verify(message, self.signer.unwrap(), signature);

            if !is_ok {
                return Err(IDOError::InvalidSignature);
            }

            // calculate IDO amount = received_value * price / 10^price_decimals
            let ido_amount = received_value.checked_mul(self.price).unwrap().checked_div((10 as u128).checked_pow(self.price_decimals).unwrap()).unwrap();
            ensure!(
                self.issued_ido_amount.checked_add(ido_amount).unwrap() <= self.max_issue_ido_amount,
                IDOError::MaxIssueIdoAmount,
            );
            self.issued_ido_amount = self.issued_ido_amount.checked_add(ido_amount).unwrap();

            let old_balances = match self.user_ido_balances.get(&self.env().caller()) {
                Some(balance) => balance,
                None => 0 as u128,
            };
            let new_balances = old_balances.checked_add(ido_amount).unwrap();
            self.user_ido_balances.insert(self.env().caller(), &new_balances);

            // emit event
            self._emit_buy_with_native_event(self.env().caller(), received_value, ido_amount, nonce);
            Ok(())
        }

        /// function to claim ido token
        #[ink(message)]
        pub fn claim_ido_token(&mut self, deadline: Timestamp, nonce: u128, amount: u128, signature: [u8; 65]) -> Result<(), IDOError> {
            ensure!(
                deadline >= self.env().block_timestamp(),
                IDOError::Expired
            );

            ensure!(
                nonce == self.account_nonce.get(&self.env().caller()).unwrap_or(0),
                IDOError::InvalidNonce(nonce.to_string())
            );

            self.account_nonce.insert(&self.env().caller(), &(nonce + 1));

            let caller = self.env().caller();
            // ensure the user has enough collateral assets
            if self.ido_token.balance_of(self.env().account_id()) < amount {
                return Err(IDOError::InsufficientBalance);
            }
            // generate message
            let message = self.gen_msg_for_claim_token(deadline, nonce, amount);

            // verify signature
            let is_ok = self._verify(message, self.signer.unwrap(), signature);

            if !is_ok {
                return Err(IDOError::InvalidSignature);
            }

            let old_balances = self.user_ido_balances.get(&self.env().caller()).unwrap_or(0);
            let new_balances = old_balances.checked_sub(amount).unwrap_or(0);
            self.user_ido_balances.insert(self.env().caller(), &new_balances);

            let result = self.ido_token.transfer(caller, amount, Vec::new());
            // check result
            if result.is_err() {
                return Err(IDOError::SafeTransferError);
            }

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
