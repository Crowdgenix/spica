#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![feature(min_specialization)]
#![allow(clippy::let_unit_value)]


#[openbrush::contract]
pub mod token_factory {
    use ink::{
        codegen::{EmitEvent, Env},
        prelude::string::String,
        prelude::vec::Vec,
        reflect::ContractEventBase,
        ToAccountId,
    };
    use openbrush::{
        contracts::{
            psp22::{
                self,
                extensions::{burnable, metadata, mintable},
                psp22::Internal,
                PSP22Error,
            },
        },
        modifiers,
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
        pub list_of_tokens: Vec<AccountId>,
    }

    impl TokenFactory {
        #[ink(constructor)]
        pub fn new(token_contract_code_hash: Hash) -> Self {
            let mut instance = Self::default();
            instance.token_contract_code_hash = token_contract_code_hash;
            instance.list_of_tokens = Vec::new();
            instance
        }

        #[ink(message)]
        pub fn get_list_of_tokens(&self) -> Vec<AccountId> {
            self.list_of_tokens.clone()
        }

        #[ink(message)]
        pub fn create_token(&mut self, owner: AccountId, name: String, symbol: String, decimals: u8, total_supply: Balance) -> Result<AccountId, TokenFactoryError> {
            let salt = (<Self as DefaultEnv>::env().block_timestamp(), b"token_contract").encode();
            let hash = xxh32(&salt, 0).to_le_bytes();

            let pool_hash = self.token_contract_code_hash;
            let pool = match TokenRef::new(owner, name, symbol, decimals, total_supply)
                .endowment(0)
                .code_hash(pool_hash)
                .salt_bytes(&hash[..4])
                .try_instantiate()
            {
                Ok(Ok(res)) => Ok(res),
                _ => Err(TokenFactoryError::CreateTokenFailed),
            }?;
            self.list_of_tokens.push(pool.to_account_id());
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
    pub struct CreateToken {
        #[ink(topic)]
        pub owner: Option<AccountId>,
        #[ink(topic)]
        pub address: Option<AccountId>,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum TokenFactoryError {
        CreateTokenFailed,
    }
}
