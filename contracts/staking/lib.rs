#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[openbrush::contract]
mod staking {
    use hex::*;
    use ink::storage::Mapping;
    use ink::{
        env::{
            hash,
        },
        codegen::{
            EmitEvent,
            Env,
        },
        prelude::vec::Vec,
        prelude::string::{String, ToString},
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
        InvalidSignature,
    }

    #[ink(storage)]
    pub struct Staking {
        owner: AccountId,
        stake_token: AccountId,
        staking_amounts: Mapping<AccountId, u128>,
        account_tiers: Mapping<AccountId, u128>,
        tier_configs: Vec<u128>,
        signer: AccountId,
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


    pub trait Internal {
        fn _emit_staking_event(&self, account: AccountId, amount: u128, new_tier: u128);
        fn _emit_unstaking_event(&self, account: AccountId, amount: u128, new_tier: u128);
    }

    impl Internal for Staking {
        fn _emit_staking_event(&self, account: AccountId, amount: u128, new_tier: u128) {
            self.env().emit_event(StakingEvent {
                staker: account,
                amount,
                new_tier,
            })
        }

        fn _emit_unstaking_event(&self, account: AccountId, amount: u128, new_tier: u128) {
            self.env().emit_event(UnstakingEvent {
                staker: account,
                amount,
                new_tier,
            })
        }
    }

    impl Staking {
        #[ink(constructor)]
        pub fn new(signer: AccountId, stake_token: AccountId, tier_configs: Vec<u128>) -> Self {
            let staking_amounts = Mapping::default();
            let account_tiers = Mapping::default();
            let owner = Self::env().caller();
            Self {
                owner,
                staking_amounts,
                account_tiers,
                stake_token,
                tier_configs,
                signer,
            }
        }

        #[ink(message)]
        pub fn set_signer(&mut self, signer: AccountId) -> Result<(), StakingError> {
            let caller = self.env().caller();
            if caller != self.owner {
                return Err(StakingError::InvalidSignature.into());
            }
            self.signer = signer;
            Ok(())
        }

        #[ink(message)]
        pub fn stake(&mut self, deadline: Timestamp, amount: u128, signature: [u8; 65]) -> Result<(), StakingError> {
            let caller = self.env().caller();

            let message = self.gen_msg_for_stake_token(deadline, amount);
            // verify signature
            let is_ok = self._verify(message, self.signer, signature);

            if !is_ok {
                return Err(StakingError::InvalidSignature);
            }

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

            self._emit_staking_event(caller, amount, tier);

            Ok(())
        }

        #[ink(message)]
        pub fn unstake(&mut self, deadline: Timestamp, amount: u128, signature: [u8; 65]) -> Result<(), StakingError> {
            let caller = self.env().caller();
            let message = self.gen_msg_for_unstake_token(deadline, amount);
            // verify signature
            let is_ok = self._verify(message, self.signer, signature);

            if !is_ok {
                return Err(StakingError::InvalidSignature);
            }

            let stake_amount = self.staking_amounts.get(&caller).unwrap_or(0);
            if stake_amount < amount {
                return Err(StakingError::InsufficientBalance)
            }
            // ensure the user has enough collateral assets
            if PSP22Ref::balance_of(&self.stake_token, self.env().account_id()) < amount {
                return Err(StakingError::InsufficientBalance)
            }

            let new_amount = self.staking_amounts.get(&caller).unwrap_or(0) - amount;
            self.staking_amounts.insert(&caller, &new_amount);

            // transfer from self to caller
            PSP22Ref::transfer_from_builder(&mut self.stake_token, caller, Self::env().account_id(), amount, Vec::<u8>::new()).call_flags(ink::env::CallFlags::default().set_allow_reentry(true)).try_invoke().map_err(|_| StakingError::TransferFailed).unwrap();

            let tier = self.get_tier_from_amount(new_amount);
            self.account_tiers.insert(&caller, &tier);

            self._emit_unstaking_event(caller, amount, tier);
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

        #[ink(message)]
        pub fn set_tiers(&mut self, tiers: Vec<u128>) {
            self.tier_configs = tiers;
        }

        #[ink(message)]
        pub fn get_tiers(&self, tiers: Vec<u128>) -> Result<Vec<u128>, StakingError> {
            Ok(self.tier_configs.clone())
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

        #[ink(message)]
        pub fn gen_msg_for_stake_token(&self, deadline: Timestamp, stake_amount: Balance) -> String {
            // generate message = buy_ido + ido_token + buyer + amount
            let mut message: String = String::from("");
            message.push_str("stake_token_");
            message.push_str(encode(&self.env().account_id()).as_str());
            message.push_str("_");
            message.push_str(encode(&self.env().caller()).as_str());
            message.push_str("_");
            message.push_str(&stake_amount.to_string().as_str());
            message.push_str("_");
            message.push_str(&deadline.to_string().as_str());
            message
        }

        #[ink(message)]
        pub fn gen_msg_for_unstake_token(&self, deadline: Timestamp, unstake_amount: Balance) -> String {
            // generate message = buy_ido + ido_token + buyer + amount
            let mut message: String = String::from("");
            message.push_str("unstake_token_");
            message.push_str(encode(&self.env().account_id()).as_str());
            message.push_str("_");
            message.push_str(encode(&self.env().caller()).as_str());
            message.push_str("_");
            message.push_str(&unstake_amount.to_string().as_str());
            message.push_str("_");
            message.push_str(&deadline.to_string().as_str());
            message
        }

        fn _verify(&self, data: String, signer: AccountId, signature: [u8; 65]) -> bool {
            ink::env::debug_println!("data {:?}", data);
            ink::env::debug_println!("signer {:?}", signer);
            ink::env::debug_println!("signature {:?}", signature);

            let mut message_hash = <hash::Blake2x256 as hash::HashOutput>::Type::default();
            ink::env::hash_bytes::<hash::Blake2x256>(&data.as_bytes(), &mut message_hash);

            ink::env::debug_println!("message_hash {:?}", message_hash);

            let output = self.env().ecdsa_recover(&signature, &message_hash).expect("Failed to recover");

            ink::env::debug_println!("pubkey {:?}", output);

            let mut signature_account_id = <hash::Blake2x256 as hash::HashOutput>::Type::default();
            ink::env::hash_encoded::<hash::Blake2x256, _>(&output, &mut signature_account_id);

            ink::env::debug_println!("Sig account id {:?}", AccountId::from(signature_account_id));

            signer == AccountId::from(signature_account_id)
        }

    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink::env::{test, debug_println, DefaultEnvironment};
        use ink::{ToAccountId};

        #[ink::test]
        fn set_tiers_works() {
            let accounts = test::default_accounts::<DefaultEnvironment>();
            let mut staking = Staking::new(accounts.alice, accounts.alice, Vec::new());
            staking.set_tiers(vec![100, 200, 300, 400, 500, 600, 700, 800, 900, 1000]);
            // assert_eq!(staking.get_tiers(c).unwrap(), vec![100, 200, 300, 400, 500, 600, 700, 800, 900, 1000]);
        }
    }
}
