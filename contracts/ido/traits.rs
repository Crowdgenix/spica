use ink::prelude::string::String;
use ink::primitives::{
    AccountId,
};

#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum IDOError {
    Custom(String),
    InvalidNonce(String), // Nonce is not valid
    MaxIssueIdoAmount, // ensure that the issued amount is less than the max_issue_ido_amount
    InvalidSignature, // ensure that the signature is valid
    SafeTransferError, // ensure that the safe transfer is successful
    CommonError,
    Expired, // ensure the deadline
    InsufficientBalance, // ensure the balance is sufficient
    Initialized, // ensure that the contract is initialized
    NotOwner, // ensure that the caller is the owner
    NotAdmin, // ensure that the caller is the admin
}
