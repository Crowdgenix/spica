use openbrush::{
    traits::{
        AccountId,
        Balance,
    },
};
use ink::prelude::string::String;
use openbrush::contracts::traits::access_control::AccessControlError;
use openbrush::traits::{Timestamp};

#[openbrush::wrapper]
pub type IdoRef = dyn Ido;

#[openbrush::trait_definition]
pub trait Ido {
    #[ink(message)]
    fn init_ido(&mut self, _ido_token: AccountId, _signer: AccountId, _price: u128, _price_decimals: u32) -> Result<(), IDOError>;

    #[ink(message)]
    fn get_ido_token(&self) -> AccountId;

    #[ink(message, payable)]
    fn buy_ido_with_native(&mut self, deadline: Timestamp, signature: [u8; 65]) -> Result<(), IDOError>;

    #[ink(message)]
    fn claim_ido_token(&mut self, deadline: Timestamp, amount: Balance, signature: [u8; 65]) -> Result<(), IDOError>;

    #[ink(message)]
    fn admin_set_price(&mut self, new_price: u128) -> Result<(), IDOError>;

    #[ink(message)]
    fn get_price(&self) -> Balance;
}

pub trait Internal {
    fn _verify(&self, data: String, signer: AccountId, signature: [u8; 65]) -> bool;
    fn _emit_buy_with_native_event(&self, _buyer: AccountId, _native_amount: Balance, _ido_token_amount: Balance);
    fn _emit_claim_token_event(&self, _buyer: AccountId, _ido_token_amount: Balance);
    fn _emit_init_ido_contract_event(&self, _ido_token: AccountId, _price: Balance, _price_decimals: u32, _signer: AccountId);
}


#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum IDOError {
    Custom(String),
    SafeTransferError,
    CommonError,
    Expired,
    InsufficientBalance,
    Initialized,
}


impl From<AccessControlError> for IDOError {
    fn from(error: AccessControlError) -> Self {
        match error {
            AccessControlError::InvalidCaller => IDOError::Custom(String::from("AC::InvalidCaller")),
            AccessControlError::MissingRole => IDOError::Custom(String::from("AC::MissingRole")),
            AccessControlError::RoleRedundant => IDOError::Custom(String::from("AC::RoleRedundant")),
        }
    }
}

