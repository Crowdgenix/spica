#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![feature(min_specialization)]

mod helpers;
pub mod traits;
mod types;
pub use traits::{*};

#[openbrush::contract]
pub mod ido {
    use hex::*;
    use ink::{
        env::{
            hash,
        },
        codegen::{
            EmitEvent,
            Env,
        },
        prelude::string::{String, ToString},
    };
    use openbrush::{
        modifiers,
        traits::Storage,
        contracts::traits::psp22::*,
    };
    use openbrush::contracts::access_control::*;
    use openbrush::traits::{DefaultEnv};
    use crate::{ensure, traits, helpers, types};
    use crate::traits::{IDOError, Internal};

    pub const SUB_ADMIN: RoleType = ink::selector_id!("SUB_ADMIN");

    #[ink(event)]
    pub struct InitIdoContract {
        #[ink(topic)]
        pub ido_token: AccountId,
        pub price: Balance,
        pub price_decimals: u32,
        pub signer: AccountId,
    }

    #[ink(event)]
    pub struct BuyTokenWithNative {
        #[ink(topic)]
        pub buyer: AccountId,
        pub native_amount: Balance,
        pub new_ido_token_amount: Balance,
    }

    #[ink(event)]
    pub struct ClaimToken {
        #[ink(topic)]
        pub buyer: AccountId,
        pub new_ido_token_amount: Balance,
    }

    #[ink(storage)]
    #[derive(Default, Storage)]
    pub struct IdoContract {
        #[storage_field]
        ido: types::Data,
        #[storage_field]
        access: access_control::Data,
        is_initialized: bool,
    }

    impl traits::Internal for IdoContract {
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

        fn _emit_buy_with_native_event(&self, _buyer: AccountId, _native_amount: Balance, _ido_token_amount: Balance) {
            self.env().emit_event(BuyTokenWithNative {
                buyer: _buyer,
                native_amount: _native_amount,
                new_ido_token_amount: _ido_token_amount,
            });
        }

        fn _emit_claim_token_event(&self, _buyer: AccountId, _ido_token_amount: Balance) {
            self.env().emit_event(ClaimToken {
                buyer: _buyer,
                new_ido_token_amount: _ido_token_amount,
            });
        }

        fn _emit_init_ido_contract_event(&self, _ido_token: AccountId, _price: Balance, _price_decimals: u32, _signer: AccountId) {
            self.env().emit_event(InitIdoContract {
                ido_token: _ido_token,
                price: _price,
                price_decimals: _price_decimals,
                signer: _signer,
            });
        }
    }


    impl traits::Ido for IdoContract {
        /// this function is initialised function, will init the contract properties
        #[ink(message)]
        fn init_ido(&mut self, _ido_token: AccountId, _signer: AccountId, _price: u128, _price_decimals: u32) -> Result<(), IDOError> {
            ensure!(self.is_initialized == false, IDOError::Initialized);
            self.ido.ido_token = _ido_token;
            self.ido.price = _price;
            self.ido.price_decimals = _price_decimals;
            self.ido.signer = _signer;
            self.is_initialized = true;

            self._emit_init_ido_contract_event(_ido_token, _price, _price_decimals, _signer);
            Ok(())
        }

        /// get ido token
        #[ink(message)]
        fn get_ido_token(&self) -> AccountId {
            self.ido.ido_token
        }

        /// function to buy ido token with native
        #[ink(message, payable)]
        fn buy_ido_with_native(&mut self, deadline: Timestamp, signature: [u8; 65]) -> Result<(), IDOError> {
            ensure!(
                deadline >= self.env().block_timestamp(),
                IDOError::Expired
            );
            let received_value = Self::env().transferred_value();

            // generate message = buy_ido + ido_token + buyer + amount
            let message = self.gen_msg_for_buy_token(deadline, received_value);

            // verify signature
            let is_ok = self._verify(message, self.ido.signer, signature);

            if !is_ok {
                return Err(IDOError::InvalidSignature);
            }

            // calculate IDO amount = received_value * price / 10^price_decimals
            let ido_amount = received_value.checked_mul(self.ido.price).unwrap().checked_div((10 as u128).checked_pow(self.ido.price_decimals).unwrap()).unwrap();
            let old_balances = match self.ido.user_ido_balances.get(&self.env().caller()) {
                Some(balance) => balance,
                None => 0 as u128,
            };
            let new_balances = old_balances.checked_add(ido_amount).unwrap();
            self.ido.user_ido_balances.insert(self.env().caller(), &new_balances);

            // emit event
            self._emit_buy_with_native_event(self.env().caller(), received_value, new_balances);
            Ok(())
        }

        /// function to claim ido token
        #[ink(message)]
        fn claim_ido_token(&mut self, deadline: Timestamp, amount: Balance, signature: [u8; 65]) -> Result<(), IDOError> {
            ensure!(
                deadline >= self.env().block_timestamp(),
                IDOError::Expired
            );
            let caller = Self::env().caller();
            // ensure the user has enough collateral assets
            if PSP22Ref::balance_of(&self.ido.ido_token, self.env().account_id()) < amount {
                return Err(IDOError::InsufficientBalance)
            }
            // generate message
            let message = self.gen_msg_for_claim_token(deadline, amount);

            // verify signature
            let is_ok = self._verify(message, self.ido.signer, signature);

            if !is_ok {
                return Err(IDOError::InvalidSignature);
            }

            let old_balances = self.ido.user_ido_balances.get(&self.env().caller()).unwrap_or(0);
            let new_balances = old_balances.checked_sub(amount).unwrap_or(0);
            self.ido.user_ido_balances.insert(self.env().caller(), &new_balances);

            let result = helpers::safe_transfer(self.ido.ido_token, caller, amount);
            // check result
            if result.is_err() {
                return Err(IDOError::SafeTransferError);
            }

            self._emit_claim_token_event(caller, new_balances);
            Ok(())
        }

        /// function to set price of ido token, only admin can call this function
        #[ink(message)]
        #[modifiers(only_role(SUB_ADMIN))]
        fn admin_set_price(&mut self, new_price: u128) -> Result<(), IDOError> {
            self.ido.price = new_price;
            Ok(())
        }

        /// function to get price of ido token
        #[ink(message)]
        fn get_price(&self) -> u128 {
            self.ido.price
        }
    }

    impl access_control::AccessControl for IdoContract {}


    impl IdoContract {
        /// constructor of IDO contract
        #[ink(constructor)]
        pub fn new(owner: AccountId) -> Self {
            let mut instance = Self::default();
            instance._init_with_admin(owner);
            instance.is_initialized = false;
            instance
        }

        /// function to get balance of ido token
        #[ink(message)]
        pub fn get_ido_token_balance(&self, account: AccountId) -> u128 {
            return self.ido.user_ido_balances.get(&account).unwrap_or(0 as u128);
        }

        // function to update code_hash (logic of IDO contract)
        #[ink(message)]
        #[modifiers(only_role(SUB_ADMIN))]
        pub fn set_code(&mut self, code_hash: [u8; 32]) -> Result<(), IDOError> {
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
        pub fn gen_msg_for_buy_token(&self, deadline: Timestamp, received_value: Balance) -> String {
            // generate message = buy_ido + ido_token + buyer + amount
            let mut message: String = String::from("");
            message.push_str("buy_ido_");
            message.push_str(encode(&self.ido.ido_token).as_str());
            message.push_str("_");
            message.push_str(encode(&self.env().caller()).as_str());
            message.push_str("_");
            message.push_str(&received_value.to_string().as_str());
            message.push_str("_");
            message.push_str(&deadline.to_string().as_str());
            message
        }

        #[ink(message)]
        pub fn gen_msg_for_claim_token(&self, deadline: Timestamp, amount: Balance) -> String {
            let mut message: String = String::from("");
            message.push_str("claim_ido_token_");
            message.push_str(encode(&self.ido.ido_token).as_str());
            message.push_str("_");
            message.push_str(encode(&self.env().caller()).as_str());
            message.push_str("_");
            message.push_str(&amount.to_string().as_str());
            message.push_str("_");
            message.push_str(&deadline.to_string().as_str());

            message
        }

        #[ink(message)]
        pub fn get_signer(&self) -> AccountId {
            self.ido.signer
        }

        #[ink(message)]
        pub fn set_signer(&mut self, _new_signer: AccountId) {
            self.ido.signer = _new_signer
        }

        #[ink(message)]
        pub fn verify_signature(&self, signature: [u8; 65], msg: String) -> bool {
            self._verify(msg, self.ido.signer, signature)
        }
    }


    #[cfg(test)]
    mod tests {
        use ink::{
            env::test::default_accounts,
            primitives::Hash,
        };
        use openbrush::traits::AccountIdExt;
        use super::*;

        #[ink::test]
        fn initialize_works() {
            let accounts = default_accounts::<ink::env::DefaultEnvironment>();
            let mut ido = IdoContract::new(accounts.alice);
            &ido.init_ido(accounts.bob, accounts.alice, 10, 1);
            assert_eq!(ido.ido.ido_token, accounts.bob);
            assert_eq!(ido.ido.price, 10);
            assert_eq!(ido.ido.price_decimals, 1);
            assert_eq!(ido.ido.signer, accounts.alice);
        }

        #[ink::test]
        fn set_signer_works() {
            let accounts = default_accounts::<ink::env::DefaultEnvironment>();
            let mut ido = IdoContract::new(accounts.alice);
            &ido.init_ido(accounts.bob, accounts.alice, 10, 1);
            &ido.set_signer(accounts.bob);
            assert_eq!(ido.ido.signer, accounts.bob);
        }

        #[ink::test]
        fn admin_set_price_works() {
            let accounts = default_accounts::<ink::env::DefaultEnvironment>();
            let mut ido = IdoContract::new(accounts.bob);
            &ido.init_ido(accounts.bob, accounts.alice, 10, 1);
            &ido.admin_set_price(20);
            assert_eq!(ido.ido.price, 10);
            ink::env::test::set_caller::<Environment>(accounts.bob);
            &ido.admin_set_price(20);
            assert_eq!(ido.ido.price, 20);
        }
    }
}
