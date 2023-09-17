use crate::types::staking::{StakingData};
use hex::*;
use ink::{
    env::{
        hash,
    },
    prelude::string::{String, ToString},
    prelude::vec::Vec,
};
use openbrush::{
    traits::{Hash, AccountId, Timestamp, Storage, DefaultEnv},
    contracts::traits::psp22::{PSP22Ref},
};

pub trait StakingInternal {
    fn _emit_staking_event(&self, account: AccountId, nonce: u128, total_amount: u128, amount: u128, new_tier: u128, timestamp: Timestamp, stake_duration: Timestamp);
    fn _emit_unstaking_event(&self, account: AccountId, nonce: u128, total_amount: u128, amount: u128, new_tier: u128, timestamp: Timestamp, fee: u128);
    fn _emit_set_tiers_event(&self, tiers: Vec<u128>);
    fn _verify(&self, data: String, signer: AccountId, signature: [u8; 65]) -> bool;
}

#[ink::trait_definition]
pub trait Staking {
    /// the function allows the owner to set the signer
    #[ink(message)]
    fn set_signer(&mut self, signer: AccountId) -> Result<(), StakingError>;

    /// function staking, after user call the API to get the signature for staking (BE API will sign the message), use will call this function to stake
    #[ink(message)]
    fn stake(&mut self, deadline: Timestamp, stake_duration: Timestamp, nonce: u128, amount: u128, signature: [u8; 65]) -> Result<(), StakingError>;

    /// function unstaking, after user call the API to get the signature for unstaking (BE API will sign the message), use will call this function to unstake
    #[ink(message)]
    fn unstake(&mut self, deadline: Timestamp, nonce: u128, amount: u128, fee: u128, signature: [u8; 65]) -> Result<(), StakingError>;

    /// function to get staking token address
    #[ink(message)]
    fn get_stake_token(&self) -> AccountId;

    #[ink(message)]
    fn get_nonce(&self) -> u128;

    /// function to get staked amount of the input account
    #[ink(message)]
    fn staking_amount_of(&self, account: AccountId) -> u128;

    /// function to get tier of the input account
    #[ink(message)]
    fn tier_of(&self, account: AccountId) -> u128;

    /// function to set list tiers of the staking contract
    #[ink(message)]
    fn set_tiers(&mut self, tiers: Vec<u128>) -> Result<(), StakingError>;

    /// function to get list tiers of the staking contract
    #[ink(message)]
    fn get_tiers(&self) -> Result<Vec<u128>, StakingError>;

    #[ink(message)]
    fn get_tier_from_amount(&self, amount: u128) -> u128;

    #[ink(message)]
    fn gen_msg_for_stake_token(&self, deadline: Timestamp, stake_duration: Timestamp, nonce: u128, stake_amount: u128) -> String;

    #[ink(message)]
    fn gen_msg_for_unstake_token(&self, deadline: Timestamp, nonce: u128, unstake_amount: u128, fee: u128) -> String;
}

#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum StakingError {
    Custom(String),
    InvalidNonce(String),
    InvalidDeadline,
    TransferFailed,
    InsufficientAllowance,
    InsufficientBalance,
    InvalidSignature,
    OnlyOwner,
}
