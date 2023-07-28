use openbrush::{
    traits::{
        AccountId,
        Balance,
    },
};
use ink::prelude::string::String;
use openbrush::traits::{Hash, Storage};
use ido::traits::{IDOError};
use openbrush::contracts::traits::access_control::AccessControlError;

#[openbrush::wrapper]
pub type FactoryRef = dyn Factory;

#[openbrush::trait_definition]
pub trait Factory {
    #[ink(message)]
    fn all_pools(&self, pid: u64) -> Option<AccountId>;

    #[ink(message)]
    fn all_pools_length(&self) -> u64;

    #[ink(message)]
    fn pool_contract_code_hash(&self) -> Hash;

    #[ink(message)]
    fn create_pool(
        &mut self,
        ido_token: AccountId,
        signer: AccountId,
        price: u128,
        price_decimals: u32,
    ) -> Result<AccountId, FactoryError>;

    #[ink(message)]
    fn get_pool(&self, token: AccountId) -> Option<AccountId>;
}

#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum FactoryError {
    // IDOError(IDOError),
    Custom(String),
    CallerIsNotFeeSetter,
    ZeroAddress,
    IdenticalAddresses,
    PoolExists,
    PoolInstantiationFailed,
    PoolInitFailed,
}

impl From<AccessControlError> for FactoryError {
    fn from(error: AccessControlError) -> Self {
        match error {
            AccessControlError::InvalidCaller => FactoryError::Custom(String::from("AC::InvalidCaller")),
            AccessControlError::MissingRole => FactoryError::Custom(String::from("AC::MissingRole")),
            AccessControlError::RoleRedundant => FactoryError::Custom(String::from("AC::RoleRedundant")),
        }
    }
}