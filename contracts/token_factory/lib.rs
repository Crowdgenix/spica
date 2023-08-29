#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![feature(min_specialization)]
#![allow(clippy::let_unit_value)]

#[openbrush::contract]
pub mod token_factory {
    use ink::{
        codegen::{EmitEvent},
        prelude::string::String,
        prelude::borrow::ToOwned,
        reflect::ContractEventBase,
        ToAccountId,
        storage::Mapping,
    };
    use openbrush::{
        traits::{Storage, DefaultEnv},
        utils::xxhash_rust::const_xxh32::xxh32,
    };
    use scale::Encode;
    use token::token::TokenRef;


    pub type Event = <TokenFactory as ContractEventBase>::Type;


    #[ink(storage)]
    #[derive(Default, Storage)]
    pub struct TokenFactory {
        pub token_contract_code_hash: Hash,
        pub tokens: Mapping<u128, AccountId>,
        pub token_length: u128,
    }

    impl TokenFactory {
        #[ink(constructor)]
        pub fn new(token_contract_code_hash: Hash) -> Self {
            let mut instance = Self::default();
            instance.token_contract_code_hash = token_contract_code_hash;
            instance.tokens = Mapping::default();
            instance.token_length = 0;
            instance
        }

        #[ink(message)]
        pub fn get_token(&self, index: u128) -> Option<AccountId> {
            self.tokens.get(&index)
        }

        #[ink(message)]
        pub fn get_token_length(&self) -> u128 {
            self.token_length
        }

        #[ink(message)]
        pub fn create_token(&mut self, owner: AccountId, name: String, symbol: String, decimals: u8, total_supply: Balance, is_require_whitelist: bool, tax_fee: Balance, document: String) -> Result<AccountId, TokenFactoryError> {
            let salt = (<Self as DefaultEnv>::env().block_timestamp(), b"token_contract").encode();
            let hash = xxh32(&salt, 0).to_le_bytes();

            let pool_hash = self.token_contract_code_hash;
            let pool = TokenRef::new(owner, name.clone(), symbol.clone(), decimals, total_supply, is_require_whitelist, tax_fee, document.clone())
                .endowment(0)
                .code_hash(pool_hash)
                .salt_bytes(&hash[..4])
                .try_instantiate().map_err(|_| TokenFactoryError::CreateTokenFailed).unwrap().unwrap();

            let index = self.token_length;
            self.tokens.insert(index, &pool.to_account_id());
            self.token_length = index + 1;
            TokenFactory::emit_event(self.env(), Event::TokenCreatedEvent(TokenCreatedEvent { owner, address: pool.to_account_id(), name, symbol, decimals, total_supply, is_require_whitelist, tax_fee, document, length: index + 1 }));
            Ok(pool.to_account_id())
        }

        // Emit event abstraction. Otherwise ink! deserializes events incorrectly when there are events from more than one contract.
        pub fn emit_event<EE: EmitEvent<Self>>(emitter: EE, event: Event) {
            emitter.emit_event(event);
        }
    }

    /// Event emitted when a token transfer occurs.
    #[ink(event)]
    #[derive(Debug)]
    pub struct TokenCreatedEvent {
        #[ink(topic)]
        pub owner: AccountId,
        #[ink(topic)]
        pub address: AccountId,
        pub name: String,
        pub symbol: String,
        pub decimals: u8,
        pub total_supply: u128,
        pub is_require_whitelist: bool,
        pub tax_fee: u128,
        pub document: String,
        pub length: u128,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum TokenFactoryError {
        CreateTokenFailed,
    }
}
