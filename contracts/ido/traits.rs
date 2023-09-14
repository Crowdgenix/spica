use ink::prelude::string::String;
use ink::primitives::{
    AccountId,
};

#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum IDOError {
    Custom(String),
    InvalidNonce(String),
    MaxIssueIdoAmount,
    InvalidSignature,
    SafeTransferError,
    CommonError,
    Expired,
    InsufficientBalance,
    Initialized,
    NotOwner,
    NotAdmin,
}


#[macro_export]
macro_rules! ensure {
    ( $x:expr, $y:expr $(,)? ) => {{
        if !$x {
            return Err($y.into())
        }
    }};
}
