use ink::prelude::string::{String};

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
    NotDeployer,
    NotOwner,
}

#[macro_export]
macro_rules! ensure {
    ( $x:expr, $y:expr $(,)? ) => {{
        if !$x {
            return Err($y.into())
        }
    }};
}