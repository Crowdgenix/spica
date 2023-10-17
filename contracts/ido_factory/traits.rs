use ink::prelude::string::{String};

#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum FactoryError {
    Custom(String),
    PoolInstantiationFailed, // ensure create IDO contract succeeds
    NotDeployer, // only deployer can deploy the IDO contract
    NotOwner, // only owner
}
