#![cfg_attr(not(feature = "std"), no_std)]
#![feature(min_specialization)]

#[openbrush::contract]
pub mod factory {
    use ink::{
        codegen::{
            EmitEvent,
        },
        ToAccountId,
    };
    use ink::env::hash::Blake2x256;
    use openbrush::modifiers;
    use openbrush::contracts::access_control::*;
    use openbrush::traits::{Storage};
    use openbrush::utils::xxhash_rust::const_xxh32::xxh32;
    use scale::Encode;

    use crate::{ensure};
    use crate::traits::{self, *};
    use crate::types::{self, *};
    use ido::traits::{IdoRef};
    use ido::ido::{IdoContractRef};

    pub const DEPLOYER: RoleType = ink::selector_id!("DEPLOYER");

    #[ink(event)]
    pub struct PoolCreated {
        #[ink(topic)]
        pub token: AccountId,
        pub pool: AccountId,
        pub pool_len: u64,
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
        fn all_pools(&self, pid: u64) -> Option<AccountId> {
            self.factory.all_pools.get(pid as usize).cloned()
        }

        #[ink(message)]
        fn all_pools_length(&self) -> u64 {
            self.factory.all_pools.len() as u64
        }

        #[ink(message)]
        fn pool_contract_code_hash(&self) -> Hash {
            self.factory.pool_contract_code_hash
        }

        #[ink(message)]
        #[modifiers(only_role(DEPLOYER))]
        fn create_pool(&mut self, ido_token: AccountId, signer: AccountId, price: u128, price_decimals: u32) -> Result<AccountId, FactoryError> {
            ensure!(
                self.factory.get_pool.get(&ido_token)
                    .is_none(),
                FactoryError::PoolExists
            );

            let pool_contract = self._instantiate_pool()?;

            let result = IdoRef::init_ido(&pool_contract, ido_token, signer, price, price_decimals);
            if result.is_err() {
                return Err(FactoryError::PoolInitFailed);
            }

            self.factory
                .get_pool
                .insert(&ido_token, &pool_contract);
            self.factory.all_pools.push(pool_contract);

            self._emit_create_pool_event(
                ido_token ,
                pool_contract,
                self.all_pools_length(),
            );

            Ok(pool_contract)
        }

        #[ink(message)]
        fn get_pool(&self, token: AccountId) -> Option<AccountId> {
            self.factory.get_pool.get(&token)
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
            let pool = match IdoContractRef::new(self.env().caller())
                .endowment(0)
                .code_hash(pool_hash)
                .salt_bytes(&hash[..4])
                .try_instantiate()
            {
                Ok(Ok(res)) => Ok(res),
                _ => Err(FactoryError::PoolInstantiationFailed),
            }?;
            Ok(pool.to_account_id())
        }

        fn _emit_create_pool_event(
            &self,
            token: AccountId,
            pool: AccountId,
            pool_len: u64,
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
            let pool_address = factory.create_pool(accounts.alice, accounts.alice, 100, 10).unwrap();
        }
    }
}
