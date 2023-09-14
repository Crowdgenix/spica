use ink::prelude::vec::Vec;
use openbrush::{
    contracts::psp22::{PSP22Error, PSP22Ref},
    traits::{AccountId, Balance},
};

#[inline]
pub fn safe_transfer(mut token: AccountId, to: AccountId, value: Balance) -> Result<(), PSP22Error> {
    PSP22Ref::transfer(&mut token, to, value, Vec::new())
}

#[inline]
pub fn safe_transfer_from(
    mut token: AccountId,
    from: AccountId,
    to: AccountId,
    value: Balance,
) -> Result<(), PSP22Error> {
    PSP22Ref::transfer_from(&mut token, from, to, value, Vec::new())
}

#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum TransferHelperError {
    TransferFailed,
}
