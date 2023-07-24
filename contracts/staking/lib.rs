#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod staking {
    use ink::storage::Mapping;
    use token::{TokenRef};
    use ink::{
        env::{
            call::{build_call, ExecutionInput, FromAccountId, Selector},
            DefaultEnvironment, Error as InkEnvError,
        },
        LangError,
        prelude::vec::Vec
    };
    use openbrush::{
        contracts::traits::psp22::{PSP22Ref},
    };

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum StakingError {
        TransferFailed,
        InsufficientAllowance,
        InsufficientBalance,
    }

    #[ink(storage)]
    pub struct Staking {
        owner: AccountId,
        stake_token: AccountId,
        staking_amounts: Mapping<AccountId, u128>,
        account_tiers: Mapping<AccountId, u128>,
        tier_configs: Vec<u128>,
    }

    #[ink(event)]
    pub struct StakingEvent {
        pub staker: AccountId,
        pub amount: u128,
        pub new_tier: u128,
    }

    #[ink(event)]
    pub struct UnstakingEvent {
        pub staker: AccountId,
        pub amount: u128,
        pub new_tier: u128,
    }

    impl Staking {
        #[ink(constructor)]
        pub fn new(stake_token: AccountId, tier_configs: Vec<u128>) -> Self {
            let staking_amounts = Mapping::default();
            let account_tiers = Mapping::default();
            let owner = Self::env().caller();
            Self {
                owner,
                staking_amounts,
                account_tiers,
                stake_token,
                tier_configs,
            }
        }

        #[ink(message)]
        pub fn stake(&mut self, amount: u128) -> Result<(), StakingError> {
            let caller = self.env().caller();

            if PSP22Ref::allowance(&self.stake_token, caller, self.env().account_id()) < amount {
                return Err(StakingError::InsufficientAllowance)
            }
            // ensure the user has enough collateral assets
            if PSP22Ref::balance_of(&self.stake_token, caller) < amount {
                return Err(StakingError::InsufficientBalance)
            }

            // transfer from caller to self
            PSP22Ref::transfer_from_builder(&mut self.stake_token, caller, Self::env().account_id(), amount, Vec::<u8>::new()).call_flags(ink::env::CallFlags::default().set_allow_reentry(true)).try_invoke().map_err(|_| StakingError::TransferFailed).unwrap();

            let new_amount = self.staking_amounts.get(&caller).unwrap_or(0) + amount;
            self.staking_amounts.insert(&caller, &new_amount);

            let tier = self.get_tier_from_amount(new_amount);
            self.account_tiers.insert(&caller, &tier);

            Ok(())
        }

        #[ink(message)]
        pub fn unstake(&mut self, amount: u128) -> Result<(), StakingError> {
            let caller = Self::env().caller();
            let stake_amount = self.staking_amounts.get(&caller).unwrap_or(0);
            if stake_amount < amount {
                return Err(StakingError::InsufficientBalance)
            }
            // ensure the user has enough collateral assets
            if PSP22Ref::balance_of(&self.stake_token, Self::env().account_id()) < amount {
                return Err(StakingError::InsufficientBalance)
            }

            let new_amount = self.staking_amounts.get(&caller).unwrap_or(0) - amount;
            self.staking_amounts.insert(&caller, &new_amount);

            // transfer from self to caller
            PSP22Ref::transfer_from_builder(&mut self.stake_token, caller, Self::env().account_id(), amount, Vec::<u8>::new()).call_flags(ink::env::CallFlags::default().set_allow_reentry(true)).try_invoke().map_err(|_| StakingError::TransferFailed).unwrap();

            let tier = self.get_tier_from_amount(new_amount);
            self.account_tiers.insert(&caller, &tier);

            Ok(())
        }

        #[ink(message)]
        pub fn get_stake_token(&self) -> AccountId {
            self.stake_token
        }

        #[ink(message)]
        pub fn get_owner(&self) -> AccountId {
            self.owner
        }

        #[ink(message)]
        pub fn set_owner(&mut self, new_owner: AccountId) {
            self.owner = new_owner
        }

        #[ink(message)]
        pub fn staking_amount_of(&self, account: AccountId) -> u128 {
            match self.staking_amounts.get(&account) {
                Some(value) => value,
                None => 0,
            }
        }

        #[ink(message)]
        pub fn tier_of(&self, account: AccountId) -> u128 {
            match self.account_tiers.get(&account) {
                Some(value) => value,
                None => 0,
            }
        }

        fn get_tier_from_amount(&self, amount: u128) -> u128 {
            let mut tier: u128 = 0;
            for i in 0..self.tier_configs.len() {
                if amount >= self.tier_configs[i] {
                    tier = i as u128;
                }
            }

            return tier;
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink::env::{test, debug_println, DefaultEnvironment};
        use ink::{ToAccountId};
        use token::{
            TokenRef,
        };

        #[ink::test]
        fn test() {
            // let accounts = test::default_accounts::<DefaultEnvironment>();
            // debug_println!("Hello, world!");
            // let token = TokenRef::new("TEST".to_string(), "TEST".to_string(), 10, 100000000);
            // debug_println!("`{:?}` is token address", token.env().account_id());
            // assert_eq!(token.total_supply(), 100000000);

            // stake_token: AccountId, tier_configs: Vec<u128>
            // let staking = Staking::new(token_address, vec![100, 200, 300, 400, 500, 600, 700, 800, 900, 1000]);
            // assert_eq!(staking.get_owner(), token_address);
        }
    }
}
//
// pub trait Internal {
//     fn _emit_staking_event(&mut self, account: AccountId, amount: u128, new_tier: u128);
//     fn _emit_unstaking_event(&mut self, account: AccountId, amount: u128, new_tier: u128);
// }
//
// impl Internal for Staking {
//     fn _emit_staking_event(&mut self, account: AccountId, amount: u128, new_tier: u128) {
//         self.env().emit_event(StakingEvent {
//             staker: account,
//             amount,
//             new_tier,
//         })
//     }
//
//     fn _emit_unstaking_event(&mut self, account: AccountId, amount: u128, new_tier: u128) {
//         self.env().emit_event(UnstakingEvent {
//             staker: account,
//             amount,
//             new_tier,
//         })
//     }
// }