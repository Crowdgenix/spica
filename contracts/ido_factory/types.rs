use ink::storage::Mapping;
use ink::prelude::vec::Vec;
use openbrush::{
    traits::{
        AccountId,
        ZERO_ADDRESS,
    },
};
use openbrush::traits::{Balance, Hash};


pub const STORAGE_KEY: u32 = openbrush::storage_unique_key!(Data);

#[derive(Debug)]
#[openbrush::upgradeable_storage(STORAGE_KEY)]
pub struct FactoryData {
    pub get_pool: Mapping<AccountId, AccountId>, // token => address
    pub all_pools: Vec<AccountId>,
    pub pool_contract_code_hash: Hash,
}

impl Default for FactoryData {
    fn default() -> Self {
        Self {
            get_pool: Mapping::default(),
            all_pools: Vec::new(),
            pool_contract_code_hash: Default::default(),
        }
    }
}
