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
        InvalidNonce(String),
        InvalidDeadline,
        TransferFailed,
        InsufficientAllowance,
        InsufficientBalance,
        InvalidSignature,
        OnlyOwner,
    }

    #[ink(storage)]
    pub struct Staking {
        owner: AccountId,
        stake_token: AccountId,
        staking_amounts: Mapping<AccountId, u128>,
        account_tiers: Mapping<AccountId, u128>,
        tier_configs: Vec<u128>,
        account_nonce: Mapping<AccountId, u128>,
        signer: AccountId,
    }

    #[ink(event)]
    pub struct StakingEvent {
        pub staker: AccountId,
        pub amount: u128,
        pub new_tier: u128,
        pub timestamp: Timestamp,
        pub nonce: u128,
    }

    #[ink(event)]
    pub struct UnstakingEvent {
        pub staker: AccountId,
        pub amount: u128,
        pub new_tier: u128,
        pub timestamp: Timestamp,
        pub nonce: u128,
    }

    #[ink(event)]
    pub struct SetTiersEvent {
        pub tiers: Vec<u128>,
    }

    pub trait Internal {
        fn _emit_staking_event(&self, account: AccountId, nonce: u128, amount: u128, new_tier: u128, timestamp: Timestamp);
        fn _emit_unstaking_event(&self, account: AccountId, nonce: u128, amount: u128, new_tier: u128, timestamp: Timestamp);
        fn _emit_set_tiers_event(&self, tiers: Vec<u128>);
    }

    impl Internal for Staking {
        fn _emit_staking_event(&self, account: AccountId, nonce: u128, amount: u128, new_tier: u128, timestamp: Timestamp) {
            self.env().emit_event(StakingEvent {
                staker: account,
                amount,
                new_tier,
                timestamp,
                nonce,
            })
        }

        fn _emit_unstaking_event(&self, account: AccountId, nonce: u128, amount: u128, new_tier: u128, timestamp: Timestamp) {
            self.env().emit_event(UnstakingEvent {
                staker: account,
                amount,
                new_tier,
                timestamp,
                nonce,
            })
        }

        fn _emit_set_tiers_event(&self, tiers: Vec<u128>) {
            self.env().emit_event(SetTiersEvent {
                tiers,
            })
        }
    }

    impl Staking {
        /// constructor for staking, admin enter the signer, token for staking and list of tiers
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
                account_nonce: Mapping::default(),
                tier_configs,
                signer,
            }
        }

        /// the function allows the owner to set the signer
        #[ink(message)]
        pub fn set_signer(&mut self, signer: AccountId) -> Result<(), StakingError> {
            let caller = self.env().caller();
            if caller != self.owner {
                return Err(StakingError::OnlyOwner.into());
            }
            self.signer = signer;
            Ok(())
        }

        /// function staking, after user call the API to get the signature for staking (BE API will sign the message), use will call this function to stake
        #[ink(message)]
        pub fn stake(&mut self, deadline: Timestamp, nonce: u128, amount: u128, signature: [u8; 65]) -> Result<(), StakingError> {
            let caller = self.env().caller();
            if deadline < self.env().block_timestamp() {
                return Err(StakingError::InvalidDeadline);
            }
            if nonce != self.account_nonce.get(&caller).unwrap_or(0) {
                return Err(StakingError::InvalidNonce(nonce.to_string()));
            }
            self.account_nonce.insert(&caller, &(nonce + 1));

            let message = self.gen_msg_for_stake_token(deadline, nonce, amount);
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

            self._emit_staking_event(caller, nonce, new_amount, tier, deadline);

            Ok(())
        }

        /// function unstaking, after user call the API to get the signature for unstaking (BE API will sign the message), use will call this function to unstake
        #[ink(message)]
        pub fn unstake(&mut self, deadline: Timestamp, nonce: u128, amount: u128, signature: [u8; 65]) -> Result<(), StakingError> {
            let caller = self.env().caller();
            if deadline < self.env().block_timestamp() {
                return Err(StakingError::InvalidDeadline);
            }
            if nonce != self.account_nonce.get(&caller).unwrap_or(0) {
                return Err(StakingError::InvalidNonce(nonce.to_string()));
            }

            self.account_nonce.insert(&caller, &(nonce + 1));
            let message = self.gen_msg_for_unstake_token(deadline, nonce, amount);
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

            self._emit_unstaking_event(caller, nonce, new_amount, tier, deadline);
            Ok(())
        }

        /// function to get staking token address
        #[ink(message)]
        pub fn get_stake_token(&self) -> AccountId {
            self.stake_token
        }

        #[ink(message)]
        pub fn get_nonce(&self) -> u128 {
            let caller = self.env().caller();
            self.account_nonce.get(&caller).unwrap_or(0)
        }

        /// function to get the owner of the staking contract
        #[ink(message)]
        pub fn get_owner(&self) -> AccountId {
            self.owner
        }

        /// function to set the owner of the staking contract
        #[ink(message)]
        pub fn set_owner(&mut self, new_owner: AccountId) -> Result<(), StakingError> {
            let caller = self.env().caller();
            if caller != self.owner {
                return Err(StakingError::OnlyOwner.into());
            }
            self.owner = new_owner;
            Ok(())
        }

        /// function to get staked amount of the input account
        #[ink(message)]
        pub fn staking_amount_of(&self, account: AccountId) -> u128 {
            self.staking_amounts.get(&account).unwrap_or(0)
        }

        /// function to get tier of the input account
        #[ink(message)]
        pub fn tier_of(&self, account: AccountId) -> u128 {
            self.account_tiers.get(&account).unwrap_or(0)
        }

        /// function to set list tiers of the staking contract
        #[ink(message)]
        pub fn set_tiers(&mut self, tiers: Vec<u128>) -> Result<(), StakingError> {
            let caller = self.env().caller();
            if caller != self.owner {
                return Err(StakingError::OnlyOwner.into());
            }
            self.tier_configs = tiers.clone();
            self._emit_set_tiers_event(tiers.clone());
            Ok(())
        }

        /// function to get list tiers of the staking contract
        #[ink(message)]
        pub fn get_tiers(&self) -> Result<Vec<u128>, StakingError> {
            Ok(self.tier_configs.clone())
        }

        /// function to update the contract code hash, use for proxy
        #[ink(message)]
        pub fn set_code(&mut self, code_hash: [u8; 32]) -> Result<(), StakingError> {
            let caller = self.env().caller();
            if caller != self.owner {
                return Err(StakingError::OnlyOwner.into());
            }
            ink::env::set_code_hash(&code_hash).unwrap_or_else(|err| {
                panic!(
                    "Failed to `set_code_hash` to {:?} due to {:?}",
                    code_hash, err
                )
            });
            ink::env::debug_println!("Switched code hash to {:?}.", code_hash);
            Ok(())
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
        pub fn gen_msg_for_stake_token(&self, deadline: Timestamp, nonce: u128, stake_amount: u128) -> String {
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
            message.push_str("_");
            message.push_str(&nonce.to_string().as_str());
            message
        }

        #[ink(message)]
        pub fn gen_msg_for_unstake_token(&self, deadline: Timestamp, nonce: u128, unstake_amount: u128) -> String {
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
            message.push_str("_");
            message.push_str(&nonce.to_string().as_str());
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
