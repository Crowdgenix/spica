use ink::storage::Mapping;
use openbrush::{
    traits::{
        AccountId,
        ZERO_ADDRESS,
    },
};
use openbrush::traits::{Balance};


pub const STORAGE_KEY: u32 = openbrush::storage_unique_key!(Data);

#[derive(Debug)]
#[openbrush::upgradeable_storage(STORAGE_KEY)]
pub struct Data {
    pub ido_token: AccountId,
    pub price: u128,
    pub price_decimals: u32,
    pub signer: AccountId,
    pub account_nonce: Mapping<AccountId, u128>,
    pub user_ido_balances: Mapping<AccountId, Balance>,
    pub max_issue_ido_amount: u128,
    pub issued_ido_amount: u128,
}

impl Default for Data {
    fn default() -> Self {
        Self {
            ido_token: ZERO_ADDRESS.into(),
            price: 0,
            signer: ZERO_ADDRESS.into(),
            price_decimals: 5,
            account_nonce: Mapping::default(),
            user_ido_balances: Mapping::new(),
            max_issue_ido_amount: 0,
            issued_ido_amount: 0,
        }
    }
}
