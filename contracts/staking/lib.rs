#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![feature(min_specialization)]
#![allow(clippy::let_unit_value)]


#[ink::contract]
pub mod staking {
    use hex::*;
    use logics::traits::staking::*;
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
        storage::Mapping,
        ToAccountId,
    };
    use ink::reflect::ContractEventBase;
    use logics::ensure;
    use token::token::TokenRef;
    use logics::traits::token::PSP22;
    pub const ZERO_ADDRESS: [u8; 32] = [255; 32];
    type Event = <StakingContract as ContractEventBase>::Type;

    #[ink(storage)]
    pub struct StakingContract {
        stake_token: TokenRef,
        staking_amounts: Mapping<AccountId, u128>,
        account_tiers: Mapping<AccountId, u128>,
        tier_configs: Vec<u128>,
        account_nonce: Mapping<AccountId, u128>,
        signer: AccountId,
        // ownable
        owner: AccountId,
    }


    impl Staking for StakingContract {
        /// the function allows the owner to set the signer
        #[ink(message)]
        fn set_signer(&mut self, signer: AccountId) -> Result<(), StakingError> {
            self._require_owner()?;

            self.signer = signer;
            Ok(())
        }

        /// function staking, after user call the API to get the signature for staking (BE API will sign the message), use will call this function to stake
        #[ink(message)]
        fn stake(&mut self, deadline: Timestamp, stake_duration: Timestamp, nonce: u128, amount: u128, signature: [u8; 65]) -> Result<(), StakingError> {
            let caller = self.env().caller();
            let me = self.env().account_id();
            ensure!(deadline >= self.env().block_timestamp(), StakingError::InvalidDeadline);
            ensure!(nonce == self.account_nonce.get(&caller).unwrap_or(0), StakingError::InvalidNonce(nonce.to_string()));

            self.account_nonce.insert(&caller, &(nonce.wrapping_add(1)));

            let message = self.gen_msg_for_stake_token(deadline, stake_duration, nonce, amount);
            // verify signature
            let is_ok = self._verify(message, self.signer, signature);
            ensure!(is_ok, StakingError::InvalidSignature);
            ensure!(self.stake_token.allowance(caller, me) >= amount, StakingError::InsufficientAllowance);
            ensure!(self.stake_token.balance_of(caller) >= amount, StakingError::InsufficientBalance);

            // transfer from caller to self
            let result = self.stake_token.transfer_from(caller, me, amount, Vec::new());
            ensure!(!result.is_err(), StakingError::TransferFailed);

            let new_amount = self.staking_amounts.get(&caller).unwrap_or(0).wrapping_add(amount);
            self.staking_amounts.insert(&caller, &new_amount);

            let tier = self.get_tier_from_amount(new_amount).unwrap_or(0);
            self.account_tiers.insert(&caller, &tier);

            self._emit_staking_event(caller, nonce, new_amount, amount, tier,deadline, stake_duration);

            Ok(())
        }

        /// function unstaking, after user call the API to get the signature for unstaking (BE API will sign the message), use will call this function to unstake
        #[ink(message)]
        fn unstake(&mut self, deadline: Timestamp, nonce: u128, amount: u128, fee: u128, signature: [u8; 65]) -> Result<(), StakingError> {
            let caller = self.env().caller();
            let me = self.env().account_id();
            ensure!(deadline >= self.env().block_timestamp(), StakingError::InvalidDeadline);
            ensure!(nonce == self.account_nonce.get(&caller).unwrap_or(0), StakingError::InvalidNonce(nonce.to_string()));

            self.account_nonce.insert(&caller, &(nonce.wrapping_add(1)));
            let message = self.gen_msg_for_unstake_token(deadline, nonce, amount, fee);
            // verify signature
            let is_ok = self._verify(message, self.signer, signature);

            ensure!(is_ok, StakingError::InvalidSignature);

            let stake_amount = self.staking_amounts.get(&caller).unwrap_or(0);
            ensure!(stake_amount >= amount, StakingError::InsufficientStakingAmount);

            // ensure the user has enough collateral assets
            ensure!(self.stake_token.balance_of(me) >= amount, StakingError::InsufficientBalance);

            let new_amount = self.staking_amounts.get(&caller).unwrap_or(0).wrapping_sub(amount);
            self.staking_amounts.insert(&caller, &new_amount);

            // transfer from self to caller
            let result = self.stake_token.transfer(caller, amount.checked_sub(fee).unwrap_or(0), Vec::<u8>::new());
            // check result
            ensure!(!result.is_err(), StakingError::TransferFailed);

            let tier = self.get_tier_from_amount(new_amount).unwrap_or(0);
            self.account_tiers.insert(&caller, &tier);

            self._emit_unstaking_event(caller, nonce, new_amount, amount, tier, deadline, fee);
            Ok(())
        }

        /// function to get staking token address
        #[ink(message)]
        fn get_stake_token(&self) -> AccountId {
            self.stake_token.to_account_id()
        }

        #[ink(message)]
        fn get_nonce(&self) -> u128 {
            let caller = self.env().caller();
            self.account_nonce.get(&caller).unwrap_or(0)
        }

        /// function to get staked amount of the input account
        #[ink(message)]
        fn staking_amount_of(&self, account: AccountId) -> u128 {
            self.staking_amounts.get(&account).unwrap_or(0)
        }

        /// function to get tier of the input account
        #[ink(message)]
        fn tier_of(&self, account: AccountId) -> u128 {
            self.account_tiers.get(&account).unwrap_or(0)
        }

        /// function to set list tiers of the staking contract
        #[ink(message)]
        // #[modifiers(only_owner)]
        fn set_tiers(&mut self, tiers: Vec<u128>) -> Result<(), StakingError> {
            self._require_owner()?;
            self.tier_configs = tiers.clone();
            self._emit_set_tiers_event(tiers.clone());
            Ok(())
        }

        /// function to get list tiers of the staking contract
        #[ink(message)]
        fn get_tiers(&self) -> Result<Vec<u128>, StakingError> {
            Ok(self.tier_configs.clone())
        }

        #[ink(message)]
        fn get_tier_from_amount(&self, amount: u128) -> Option<u128> {
            for (i, tier_config) in self.tier_configs.iter().enumerate() {
                if amount >= *tier_config {
                    return Some(i as u128);
                }
            }
            None
        }

        #[ink(message)]
        fn gen_msg_for_stake_token(&self, deadline: Timestamp, stake_duration: Timestamp, nonce: u128, stake_amount: u128) -> String {
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
            message.push_str(&stake_duration.to_string().as_str());
            message.push_str("_");
            message.push_str(&nonce.to_string().as_str());
            message
        }

        #[ink(message)]
        fn gen_msg_for_unstake_token(&self, deadline: Timestamp, nonce: u128, unstake_amount: u128, fee: u128) -> String {
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
            message.push_str("_");
            message.push_str(&fee.to_string().as_str());
            message
        }
    }

    impl StakingInternal for StakingContract {
        fn _emit_staking_event(&self, account: AccountId, nonce: u128, total_amount: u128, amount: u128, new_tier: u128, timestamp: Timestamp, stake_duration: Timestamp) {
            Self::emit_event(self.env(), Event::StakingEvent(StakingEvent {
                staker: account,
                total_amount,
                amount,
                new_tier,
                timestamp,
                nonce,
                stake_duration,
            }))
        }

        fn _emit_unstaking_event(&self, account: AccountId, nonce: u128, total_amount: u128, amount: u128, new_tier: u128, timestamp: Timestamp, fee: u128) {
            Self::emit_event(self.env(), Event::UnstakingEvent(UnstakingEvent {
                staker: account,
                total_amount,
                amount,
                new_tier,
                timestamp,
                nonce,
                fee,
            }))
        }

        fn _emit_set_tiers_event(&self, tiers: Vec<u128>) {
            Self::emit_event(self.env(), Event::SetTiersEvent(SetTiersEvent {
                tiers,
            }))
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

    impl StakingContract {
        /// constructor for staking, admin enter the signer, token for staking and list of tiers
        #[ink(constructor)]
        pub fn new(signer: AccountId, stake_token: AccountId, tier_configs: Vec<u128>) -> Self {
            let caller = Self::env().caller();
            Self {
                stake_token: ink::env::call::FromAccountId::from_account_id(stake_token),
                staking_amounts: Mapping::default(),
                account_tiers: Mapping::default(),
                tier_configs: tier_configs.clone(),
                account_nonce: Mapping::default(),
                signer,
                owner: caller,
            }
        }


        #[ink(message)]
        pub fn owner(&self) -> AccountId {
            self.owner.clone()
        }

        #[ink(message)]
        pub fn transfer_ownership(&mut self, new_owner: AccountId) -> Result<(), StakingError> {
            self._require_owner()?;
            self.owner = new_owner;
            Ok(())
        }

        #[ink(message)]
        pub fn admin_collect_all_token(&mut self) -> Result<(), StakingError> {
            self._require_owner()?;
            let balance = self.stake_token.balance_of(self.env().account_id());
            self.stake_token.transfer(self.env().caller(), balance, Vec::new()).map_err(|_| StakingError::TransferFailed)?;
            Ok(())
        }

        /// function to update the contract code hash, use for proxy
        #[ink(message)]
        pub fn set_code(&mut self, code_hash: [u8; 32]) -> Result<(), StakingError> {
            self._require_owner()?;
            ink::env::set_code_hash(&code_hash).unwrap_or_else(|err| {
                panic!(
                    "Failed to `set_code_hash` to {:?} due to {:?}",
                    code_hash, err
                )
            });
            ink::env::debug_println!("Switched code hash to {:?}.", code_hash);
            Ok(())
        }

        fn _require_owner(&self) -> Result<(), StakingError> {
            ensure!(self.owner == self.env().caller(), StakingError::Custom("Not owner".to_string()));
            Ok(())
        }

        fn emit_event<EE>(emitter: EE, event: Event)
            where
                EE: EmitEvent<StakingContract>,
        {
            emitter.emit_event(event);
        }
    }


    #[ink(event)]
    #[derive(Debug)]
    pub struct StakingEvent {
        pub staker: AccountId,
        pub total_amount: u128,
        pub amount: u128,
        pub new_tier: u128,
        pub timestamp: Timestamp,
        pub stake_duration: Timestamp,
        pub nonce: u128,
    }

    #[ink(event)]
    #[derive(Debug)]
    pub struct UnstakingEvent {
        pub staker: AccountId,
        pub total_amount: u128,
        pub amount: u128,
        pub new_tier: u128,
        pub timestamp: Timestamp,
        pub nonce: u128,
        pub fee: u128,
    }

    #[ink(event)]
    pub struct SetTiersEvent {
        pub tiers: Vec<u128>,
    }
}
