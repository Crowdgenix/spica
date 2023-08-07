#![cfg_attr(not(feature = "std"), no_std)]

#[openbrush::contract]
pub mod factory {
    use ink::{
        codegen::{
            EmitEvent,
        },
        ToAccountId,
    };
    use openbrush::modifiers;
    use openbrush::contracts::access_control::*;
    use openbrush::traits::{Storage};
    use openbrush::utils::xxhash_rust::const_xxh32::xxh32;
    use scale::Encode;

    use crate::traits::{*};
    use crate::types::{*};
    use ido::traits::{IdoRef};
    use ido::ido::{IdoContractRef};

    pub const DEPLOYER: RoleType = ink::selector_id!("DEPLOYER");

    #[ink(event)]
    pub struct PoolCreated {
        #[ink(topic)]
        pub token: AccountId,
        pub pool: AccountId,
        pub pool_len: u128,
    }

    #[ink(storage)]
    #[derive(Default, Storage)]
    pub struct FactoryContract {
        #[storage_field]
        factory: FactoryData,
        #[storage_field]
        access_control: access_control::Data,
    }

    impl access_control::AccessControl for FactoryContract {}

    impl Factory for FactoryContract {
        #[ink(message)]
        fn pools_length(&self) -> u128 {
            self.factory.pool_length
        }

        #[ink(message)]
        fn pool_contract_code_hash(&self) -> Hash {
            self.factory.pool_contract_code_hash
        }

        #[ink(message)]
        #[modifiers(only_role(DEPLOYER))]
        fn create_pool(&mut self, ido_token: AccountId, signer: AccountId, price: u128, price_decimals: u32, max_issue_ido_amount: u128) -> Result<AccountId, FactoryError> {
            let pool_contract = self._instantiate_pool()?;
            IdoRef::init_ido(&pool_contract, ido_token, signer, price, price_decimals, max_issue_ido_amount).map_err(|_| FactoryError::PoolInitFailed).unwrap();

            let index = self.factory.pool_length;
            self.factory
                .pools
                .insert(&index, &pool_contract);
            self.factory.pool_length = index + 1;

            self._emit_create_pool_event(
                ido_token ,
                pool_contract,
                index + 1,
            );

            Ok(pool_contract)
        }

        #[ink(message)]
        fn pools(&self, index: u128) -> Option<AccountId> {
            self.factory.pools.get(&index)
        }
    }

    impl FactoryContract {
        #[ink(constructor)]
        pub fn new(pool_code_hash: Hash) -> Self {
            let mut instance = Self::default();
            instance.factory.pool_contract_code_hash = pool_code_hash;
            instance._init_with_admin(Self::env().caller());
            instance
        }

        fn _instantiate_pool(&mut self) -> Result<AccountId, FactoryError> {
            let salt = (self.env().block_timestamp(), b"ido_factory").encode();
            let hash = xxh32(&salt, 0).to_le_bytes();

            let pool_hash = self.factory.pool_contract_code_hash;
            let pool = IdoContractRef::new(self.env().caller())
                .endowment(0)
                .code_hash(pool_hash)
                .salt_bytes(&hash[..4])
                .try_instantiate().map_err(|_| FactoryError::PoolInstantiationFailed).unwrap().unwrap();
            Ok(pool.to_account_id())
        }

        fn _emit_create_pool_event(
            &self,
            token: AccountId,
            pool: AccountId,
            pool_len: u128,
        ) {
            EmitEvent::<FactoryContract>::emit_event(
                self.env(),
                PoolCreated {
                    token,
                    pool,
                    pool_len,
                },
            )
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
            ink::env::debug_println!("data {:?}", DEPLOYER);
            let accounts = default_accounts::<ink::env::DefaultEnvironment>();
            let mut factory = FactoryContract::new(Hash::default());
            let pool_address = factory.create_pool(accounts.alice, accounts.alice, 100, 10, 100000).unwrap();
        }
    }
}
