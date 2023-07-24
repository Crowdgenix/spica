use openbrush::{
    traits::{
        AccountId,
        Balance,
    },
};
use ink::prelude::string::String;
use openbrush::traits::{Hash, Storage, Timestamp};
use crate::ido::ido::ClaimToken;

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
    fn admin_set_price(&mut self, new_price: u128);
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
    SafeTransferError,
    CommonError,
    Expired,
    InsufficientBalance,
}
