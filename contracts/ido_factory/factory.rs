#![cfg_attr(not(feature = "std"), no_std)]

#[ink::contract]
pub mod factory {
    use ink::{
        codegen::{
            EmitEvent,
        },
        ToAccountId,
        prelude::string::{String},
        storage::{
            Mapping,
        }
    };
    use openbrush::utils::xxhash_rust::const_xxh32::xxh32;
    use scale::Encode;

    use crate::traits::{*};
    use logics::{ensure};
    use ido::ido::{IdoContractRef};

    type RoleType = u32;
    pub const DEPLOYER: RoleType = ink::selector_id!("DEPLOYER");

    #[ink(event)]
    pub struct PoolCreated {
        #[ink(topic)]
        pub token: AccountId,
        #[ink(topic)]
        pub id: u128,
        pub pool: AccountId,
        pub pool_len: u128,
    }

    #[ink(storage)]
    pub struct FactoryContract {
        pools: Mapping<u128, AccountId>,
        pool_length: u128,
        pool_contract_code_hash: Hash,
        roles: Mapping<(AccountId, RoleType), bool>,
        owner: AccountId,
    }


    impl FactoryContract {
        #[ink(constructor)]
        pub fn new(pool_code_hash: Hash) -> Self {
            Self {
                pools: Mapping::default(),
                pool_length: 0,
                pool_contract_code_hash: pool_code_hash,
                roles: Mapping::default(),
                owner: Self::env().caller(),
            }
        }

        #[ink(message)]
        pub fn pools_length(&self) -> u128 {
            self.pool_length
        }

        #[ink(message)]
        pub fn pool_contract_code_hash(&self) -> Hash {
            self.pool_contract_code_hash
        }

        #[ink(message)]
        pub fn set_pool_contract_code_hash(&mut self, new_hash: Hash) -> Result<(), FactoryError> {
            self.pool_contract_code_hash = new_hash;
            Ok(())
        }

        #[ink(message)]
        pub fn create_pool(&mut self, id: u128, ido_token: AccountId, signer: AccountId, price: u128, price_decimals: u32, max_issue_ido_amount: u128) -> Result<AccountId, FactoryError> {
            let is_deployer: bool = self.roles.get(&(self.env().caller(), DEPLOYER)).unwrap_or(false);
            ensure!(is_deployer, FactoryError::NotDeployer);

            let pool_contract = self._instantiate_pool(ido_token, signer, price, price_decimals, max_issue_ido_amount)?;

            let index = self.pool_length;
            self.pools
                .insert(&index, &pool_contract);
            self.pool_length = index + 1;

            self._emit_create_pool_event(
                id,
                ido_token ,
                pool_contract,
                index + 1,
            );

            Ok(pool_contract)
        }

        #[ink(message)]
        pub fn pools(&self, index: u128) -> Option<AccountId> {
            self.pools.get(&index)
        }


        #[ink(message)]
        pub fn owner(&self) -> AccountId {
            self.owner.clone()
        }

        #[ink(message)]
        pub fn transfer_ownership(&mut self, new_owner: AccountId) -> Result<(), FactoryError> {
            ensure!(self.env().caller() == self.owner, FactoryError::NotOwner);
            self.owner = new_owner;
            Ok(())
        }

        #[ink(message)]
        pub fn grant_role(&mut self, role: RoleType, user: AccountId) -> Result<(), FactoryError> {
            ensure!(self.env().caller() == self.owner, FactoryError::NotOwner);
            self.roles.insert(&(user, role), &true);
            Ok(())
        }

        #[ink(message)]
        pub fn revoke_role(&mut self, role: RoleType, user: AccountId) -> Result<(), FactoryError> {
            ensure!(self.env().caller() == self.owner, FactoryError::NotOwner);
            self.roles.remove(&(user, role));
            Ok(())
        }

        #[ink(message)]
        pub fn is_role_granted(&self, role: RoleType, user: AccountId) -> Result<bool, FactoryError> {
            let ok = self.roles.get(&(user, role)).unwrap_or(false);
            Ok(ok)
        }

        fn _instantiate_pool(&mut self, ido_token: AccountId, signer: AccountId, price: u128, price_decimals: u32, max_issue_ido_amount: u128) -> Result<AccountId, FactoryError> {
            let salt = (self.env().block_timestamp(), b"ido_factory").encode();
            let hash = xxh32(&salt, 0).to_le_bytes();

            let pool_hash = self.pool_contract_code_hash;
            let pool = IdoContractRef::new(self.env().caller(), ido_token, signer, price, price_decimals, max_issue_ido_amount)
                .endowment(0)
                .code_hash(pool_hash)
                .salt_bytes(&hash[..4])
                .try_instantiate().map_err(|_| FactoryError::PoolInstantiationFailed).unwrap().unwrap();
            Ok(pool.to_account_id())
        }

        fn _emit_create_pool_event(
            &self,
            id: u128,
            token: AccountId,
            pool: AccountId,
            pool_len: u128,
        ) {
            EmitEvent::<FactoryContract>::emit_event(
                self.env(),
                PoolCreated {
                    id,
                    token,
                    pool,
                    pool_len,
                },
            )
        }
    }
}
