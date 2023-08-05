use ink::storage::Mapping;
use openbrush::{
    traits::{
        AccountId,
    },
};
use openbrush::traits::{Hash};


pub const STORAGE_KEY: u32 = openbrush::storage_unique_key!(Data);

#[derive(Debug)]
#[openbrush::upgradeable_storage(STORAGE_KEY)]
pub struct FactoryData {
    pub pools: Mapping<u128, AccountId>,
    pub pool_length: u128,
    pub pool_contract_code_hash: Hash,
}

impl Default for FactoryData {
    fn default() -> Self {
        Self {
            pools: Mapping::default(),
            pool_length: 0,
            pool_contract_code_hash: Default::default(),
        }
    }
}
