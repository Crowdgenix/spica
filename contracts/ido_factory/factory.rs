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

    // this event is emitted when a new pool is deployed, we need to emit this event in the factory contract because BE crawler crawls the factory contract events
    #[ink(event)]
    pub struct PoolCreated {
        #[ink(topic)]
        pub token: AccountId, // token of the IDO contract
        #[ink(topic)]
        pub id: u128, // id of the IDO contract, storing in the mapping of this contract
        pub pool: AccountId, // the address of the IDO contract
        pub pool_len: u128, // the length of the list of created IDO contracts
    }

    #[ink(storage)]
    pub struct FactoryContract {
        pools: Mapping<u128, AccountId>, // mapping to store the address of the IDO contract, the key is the id of the IDO contract, latest id is the length of the list of created IDO contracts
        pool_length: u128, // length of the list of created IDO contracts, we use mapping instead of Vec, so we need this property
        pool_contract_code_hash: Hash, // the code hash of the IDO contract
        roles: Mapping<(AccountId, RoleType), bool>, // mapping to store the roles of users
        owner: AccountId, // the owner of the Factory contract
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

        // get the length of the list of created IDO contracts
        #[ink(message)]
        pub fn pools_length(&self) -> u128 {
            self.pool_length
        }

        // get the code hash of the IDO contract
        #[ink(message)]
        pub fn pool_contract_code_hash(&self) -> Hash {
            self.pool_contract_code_hash
        }

        // set the code hash of the IDO contract
        #[ink(message)]
        pub fn set_pool_contract_code_hash(&mut self, new_hash: Hash) -> Result<(), FactoryError> {
            ensure!(self.env().caller() == self.owner, FactoryError::NotOwner);
            self.pool_contract_code_hash = new_hash;
            Ok(())
        }

        // the deployer can deploy new IDO contract
        #[ink(message)]
        pub fn create_pool(&mut self, id: u128, ido_token: AccountId, signer: AccountId, price: u128, price_decimals: u32, max_issue_ido_amount: u128) -> Result<AccountId, FactoryError> {
            let is_deployer: bool = self.roles.get(&(self.env().caller(), DEPLOYER)).unwrap_or(false);
            // ensure the caller is the deployer
            ensure!(is_deployer, FactoryError::NotDeployer);

            // instantiate the IDO contract with the parameters
            let pool_contract = self._instantiate_pool(ido_token, signer, price, price_decimals, max_issue_ido_amount)?;

            // the index is the key of the mapping of the Factory contract, we insert key - address of the IDO contract
            let index = self.pool_length;
            self.pools
                .insert(&index, &pool_contract);
            // update the length of the list of created IDO contracts
            self.pool_length = index + 1;

            self._emit_create_pool_event(
                id,
                ido_token ,
                pool_contract,
                index + 1,
            );

            Ok(pool_contract)
        }

        // get the address of the IDO contract with the given id
        #[ink(message)]
        pub fn pools(&self, index: u128) -> Option<AccountId> {
            self.pools.get(&index)
        }

        // get the address of the owner of the Factory contract
        #[ink(message)]
        pub fn owner(&self) -> AccountId {
            self.owner.clone()
        }

        // owner can transfer the ownership of the Factory contract
        #[ink(message)]
        pub fn transfer_ownership(&mut self, new_owner: AccountId) -> Result<(), FactoryError> {
            ensure!(self.env().caller() == self.owner, FactoryError::NotOwner);
            self.owner = new_owner;
            Ok(())
        }

        // owner can grant roles to users
        #[ink(message)]
        pub fn grant_role(&mut self, role: RoleType, user: AccountId) -> Result<(), FactoryError> {
            ensure!(self.env().caller() == self.owner, FactoryError::NotOwner);
            self.roles.insert(&(user, role), &true);
            Ok(())
        }

        // owner can revoke roles to users
        #[ink(message)]
        pub fn revoke_role(&mut self, role: RoleType, user: AccountId) -> Result<(), FactoryError> {
            ensure!(self.env().caller() == self.owner, FactoryError::NotOwner);
            self.roles.remove(&(user, role));
            Ok(())
        }

        // check if the given user has the given role
        #[ink(message)]
        pub fn is_role_granted(&self, role: RoleType, user: AccountId) -> Result<bool, FactoryError> {
            let ok = self.roles.get(&(user, role)).unwrap_or(false);
            Ok(ok)
        }

        // this function is used to instantiate the IDO contract
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
