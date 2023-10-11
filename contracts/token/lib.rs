#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![feature(min_specialization)]
#![allow(clippy::let_unit_value)]

pub use crate::token::*;

/// the token follow by the BitBond Tokenisation Engine (ERC20 - PSP22 token standard)
/// we have the option for blacklist, the option whitelist, the option for charging tax fee when transferring tokens,
/// the option for add address to list ignoring the tax_fee, forcing transferring and minting tokens for owner account,
/// pausing, burning, set max allocation amount for each user
#[ink::contract]
pub mod token {
    use ink::{
        codegen::{EmitEvent, Env},
        prelude::vec::Vec,
        prelude::string::{String, ToString},
        reflect::ContractEventBase,
        storage::Mapping,
        env::transfer,
    };
    use logics::traits::token::{PSP22, PSP22Metadata, PSP22Pausable, PSP22Error};

    /// Result type
    pub type Result<T> = core::result::Result<T, PSP22Error>;

    /// Event type
    pub type Event = <Token as ContractEventBase>::Type;

    pub const ZERO_ADDRESS: [u8; 32] = [255; 32];

    #[ink(storage)]
    #[derive(Default)]
    pub struct Token {
        supply: u128,
        balances: Mapping<AccountId, u128>,
        allowances: Mapping<(AccountId, AccountId), u128>,
        // if this option is enabled, the token requires a whitelisted account to transfer tokens
        is_required_whiteList: bool,
        // if this option is enabled, the token requires a non-blacklisted account to transfer tokens
        is_required_blackList: bool,
        // if this option is enabled, users can burn their tokens
        is_burnable: bool,
        // if this option is enabled, owner can mint tokens
        is_mintable: bool,
        // if this option is enabled, owner can pause and unpause the token contract
        is_pausable: bool,
        // if this option is enabled, owner can set max allocation amount for each user
        is_require_max_alloc_per_address: bool,
        // max allocation amount for each user
        max_alloc_per_user: u128,
        // if this option is enabled, owner can force transferring tokens from any account to any account
        is_force_transfer_enable: bool,
        // list of addresses can transfer tokens if is_required_whiteList is enabled
        whitelist: Mapping<AccountId, bool>,
        // list of addresses can not transfer tokens if is_required_blackList is enabled
        blacklist: Mapping<AccountId, bool>,
        // tax fee for transferring tokens, the unit is AZERO
        tax_fee: u128,
        // this account can call claim_tax_fee to transfer the tax fee from this contract to this account
        tax_fee_receiver: Option<AccountId>,
        // document for the token contract
        document: String,
        // list of addresses ignores the tax_fee when transferring tokens
        list_ignore_from_tax_fee: Mapping<AccountId, bool>,
        // metadata
        name: Option<String>,
        symbol: Option<String>,
        decimals: u8,
        // pausable
        paused: bool,
        // ownable
        owner: Option<AccountId>,
    }

    impl PSP22Metadata for Token {
        #[ink(message)]
        fn token_name(&self) -> Option<String> {
            self.name.clone()
        }

        #[ink(message)]
        fn token_symbol(&self) -> Option<String> {
            self.symbol.clone()
        }

        #[ink(message)]
        fn token_decimals(&self) -> u8 {
            self.decimals.clone()
        }
    }

    impl PSP22Pausable for Token {
        #[ink(message)]
        fn paused(&self) -> bool {
            self.paused.clone()
        }

        #[ink(message)]
        // #[modifiers(only_owner)]
        fn change_pause_state(&mut self) -> Result<()> {
            self._require_owner()?;
            if !self.is_pausable {
                return Err(PSP22Error::Custom(String::from("not pausable")));
            }
            if self.paused() {
                self.paused = false;
            } else {
                self.paused = true;
            }
            Ok(())
        }
    }

    impl PSP22 for Token {
        #[ink(message)]
        fn total_supply(&self) -> u128 {
            self._total_supply()
        }

        #[ink(message)]
        fn balance_of(&self, owner: AccountId) -> u128 {
            self._balance_of(&owner)
        }

        #[ink(message)]
        fn allowance(&self, owner: AccountId, spender: AccountId) -> u128 {
            self._allowance(&owner, &spender)
        }

        #[ink(message, payable)]
        fn transfer(&mut self, to: AccountId, value: u128, data: Vec<u8>) -> Result<()> {
            let from = Self::env().caller();
            self._transfer_from_to(from, to, value, data)?;
            Ok(())
        }

        #[ink(message, payable)]
        fn transfer_from(
            &mut self,
            from: AccountId,
            to: AccountId,
            value: u128,
            data: Vec<u8>,
        ) -> Result<()> {
            let caller = Self::env().caller();
            let allowance = self._allowance(&from, &caller);

            if allowance < value {
                return Err(PSP22Error::InsufficientAllowance);
            }

            self._approve_from_to(from, caller, allowance - value)?;
            self._transfer_from_to(from, to, value, data)?;
            Ok(())
        }

        #[ink(message)]
        fn approve(&mut self, spender: AccountId, value: u128) -> Result<()> {
            if self.paused() {
                return Err(PSP22Error::Custom(String::from("Paused")));
            }
            let owner = Self::env().caller();
            self._approve_from_to(owner, spender, value)?;
            Ok(())
        }

        #[ink(message)]
        fn increase_allowance(&mut self, spender: AccountId, delta_value: u128) -> Result<()> {
            let owner = Self::env().caller();
            self._approve_from_to(owner, spender, self._allowance(&owner, &spender) + delta_value)
        }

        #[ink(message)]
        fn decrease_allowance(&mut self, spender: AccountId, delta_value: u128) -> Result<()> {
            let owner = Self::env().caller();
            let allowance = self._allowance(&owner, &spender);

            if allowance < delta_value {
                return Err(PSP22Error::InsufficientAllowance);
            }

            self._approve_from_to(owner, spender, allowance - delta_value)
        }
    }

    impl Token {
        #[ink(constructor)]
        pub fn new(owner: AccountId, name: String, symbol: String, decimals: u8, total_supply: u128, is_require_whitelist: bool,
                   is_require_blacklist: bool, is_burnable: bool, is_mintable: bool, is_force_transfer_enable: bool,
                   is_pausable: bool, is_require_max_alloc_per_address: bool, max_alloc_per_user: u128, tax_fee_receiver: AccountId, tax_fee: u128, document: String) -> Self {
            let mut instance = Self::default();

            instance.owner = Some(owner.clone());
            instance.name = Some(name);
            instance.symbol = Some(symbol);
            instance.decimals = decimals;
            instance.whitelist = Mapping::default();
            instance.list_ignore_from_tax_fee = Mapping::default();
            instance.is_required_whiteList = is_require_whitelist;
            instance.is_required_blackList = is_require_blacklist;
            instance.is_burnable = is_burnable;
            instance.is_mintable = is_mintable;
            instance.is_force_transfer_enable = is_force_transfer_enable;
            instance.is_pausable = is_pausable;
            instance.is_require_max_alloc_per_address = is_require_max_alloc_per_address;
            instance.max_alloc_per_user = max_alloc_per_user;
            instance.tax_fee = tax_fee;
            instance.tax_fee_receiver = Some(tax_fee_receiver);
            instance.document = document;

            // Mint initial supply to the caller.
            instance.balances.insert(&owner, &total_supply);
            instance.supply = total_supply;
            instance._emit_transfer_event(None, Some(owner), total_supply);

            instance
        }

        // Emit event abstraction. Otherwise ink! deserializes events incorrectly when there are events from more than one contract.
        pub fn emit_event<EE: EmitEvent<Self>>(emitter: EE, event: Event) {
            emitter.emit_event(event);
        }

        #[ink(message)]
        pub fn document(&self) -> String {
            self.document.clone().into()
        }

        #[ink(message)]
        pub fn tax_fee(&self) -> u128 {
            self.tax_fee
        }

        #[ink(message)]
        pub fn owner(&self) -> Option<AccountId> {
            self.owner.clone()
        }

        #[ink(message)]
        pub fn transfer_ownership(&mut self, new_owner: AccountId) -> Result<()> {
            if self.paused() {
                return Err(PSP22Error::Custom(String::from("Paused")));
            }
            self._require_owner()?;
            self.owner = Some(new_owner);
            Ok(())
        }

        #[ink(message)]
        pub fn add_account_to_list_ignore_tax_fee(&mut self, users: Vec<AccountId>) -> Result<()> {
            if self.paused() {
                return Err(PSP22Error::Custom(String::from("Paused")));
            }
            self._require_owner()?;
            for user in users {
                self.list_ignore_from_tax_fee.insert(user, &true);
            }
            Ok(())
        }

        #[ink(message)]
        pub fn remove_account_to_list_ignore_tax_fee(&mut self, users: Vec<AccountId>) -> Result<()> {
            if self.paused() {
                return Err(PSP22Error::Custom(String::from("Paused")));
            }
            self._require_owner()?;
            for user in users {
                self.list_ignore_from_tax_fee.remove(user);
            }
            Ok(())
        }

        #[ink(message)]
        // #[modifiers(only_owner)]
        pub fn add_whitelist(&mut self, users: Vec<AccountId>) -> Result<()> {
            if self.paused() {
                return Err(PSP22Error::Custom(String::from("Paused")));
            }
            self._require_owner()?;
            for user in users {
                self.whitelist.insert(user, &true);
            }
            Ok(())
        }

        #[ink(message)]
        // #[modifiers(only_owner)]
        pub fn remove_whitelist(&mut self, users: Vec<AccountId>) -> Result<()> {
            if self.paused() {
                return Err(PSP22Error::Custom(String::from("Paused")));
            }
            self._require_owner()?;
            for user in users {
                self.whitelist.remove(user);
            }
            Ok(())
        }


        #[ink(message)]
        // #[modifiers(only_owner)]
        pub fn add_blacklist(&mut self, users: Vec<AccountId>) -> Result<()> {
            if self.paused() {
                return Err(PSP22Error::Custom(String::from("Paused")));
            }
            self._require_owner()?;
            for user in users {
                self.blacklist.insert(user, &true);
            }
            Ok(())
        }

        #[ink(message)]
        // #[modifiers(only_owner)]
        pub fn remove_blacklist(&mut self, users: Vec<AccountId>) -> Result<()> {
            if self.paused() {
                return Err(PSP22Error::Custom(String::from("Paused")));
            }
            self._require_owner()?;
            for user in users {
                self.blacklist.remove(user);
            }
            Ok(())
        }

        #[ink(message)]
        /// Forcing a transfer from one account to another account. Requires the owner to call this function.
        pub fn force_transfer(&mut self, from_account: AccountId, to_account: AccountId, amount: u128) -> Result<()> {
            self._require_owner()?;
            if !self.is_force_transfer_enable {
                return Err(PSP22Error::Custom(String::from("not allow force transfer")));
            }
            self._transfer_from_to(from_account, to_account, amount, Vec::new())?;
            Ok(())
        }

        #[ink(message)]
        /// Claim the tax fee, only tax_fee_receiver can call this function.
        pub fn claim_tax_fee(&mut self, to: AccountId, amount: u128) -> Result<()> {
            if self.paused() {
                return Err(PSP22Error::Custom(String::from("Paused")));
            }
            if self.env().caller() != self.tax_fee_receiver.unwrap() {
                return Err(PSP22Error::Custom(String::from("caller is not tax_fee_receiver")));
            }
            let ok = self.env().transfer(to, amount);
            if ok.is_err() {
                return Err(PSP22Error::Custom(String::from("Error while transfer native")));
            }
            Ok(())
        }

        #[ink(message)]
        /// Burns the `amount` of underlying tokens from the balance of `account` recipient.
        pub fn burn(&mut self, amount: u128) -> Result<()> {
            if !self.is_burnable {
                return Err(PSP22Error::Custom(String::from("not burnable")));
            }
            self._burn_from(self.env().caller(), amount)
        }

        #[ink(message)]
        pub fn set_code(&mut self, code_hash: [u8; 32]) -> Result<()> {
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

        #[ink(message)]
        // owner can mint token with this function.
        pub fn mint(&mut self, account: AccountId, amount: u128) -> Result<()> {
            if self.paused() {
                return Err(PSP22Error::Custom(String::from("Paused")));
            }
            self._require_owner()?;
            if !self.is_mintable {
                return Err(PSP22Error::Custom(String::from("not mintable")));
            }
            self._mint_to(account, amount)
        }

        fn _total_supply(&self) -> u128 {
            self.supply.clone()
        }

        fn _balance_of(&self, owner: &AccountId) -> u128 {
            self.balances.get(owner).unwrap_or(0)
        }

        fn _allowance(&self, owner: &AccountId, spender: &AccountId) -> u128 {
            self.allowances.get((owner, spender)).unwrap_or(0)
        }

        fn _transfer_from_to(
            &mut self,
            from: AccountId,
            to: AccountId,
            amount: u128,
            _data: Vec<u8>,
        ) -> Result<()> {
            let from_balance = self._balance_of(&from);

            if from_balance < amount {
                return Err(PSP22Error::InsufficientBalance);
            }

            self._before_token_transfer(Some(&from), Some(&to), &amount)?;

            self.balances.insert(&from, &(from_balance - amount));

            let to_balance = self._balance_of(&to);
            self.balances.insert(&to, &(to_balance + amount));

            self._after_token_transfer(Some(&from), Some(&to), &amount)?;
            self._emit_transfer_event(Some(from), Some(to), amount);

            Ok(())
        }

        fn _approve_from_to(
            &mut self,
            owner: AccountId,
            spender: AccountId,
            amount: u128,
        ) -> Result<()> {
            if self.paused() {
                return Err(PSP22Error::Custom(String::from("Paused")));
            }

            self.allowances.insert(&(owner, spender), &amount);
            self._emit_approval_event(owner, spender, amount);
            Ok(())
        }

        fn _mint_to(&mut self, account: AccountId, amount: u128) -> Result<()> {
            self._before_token_transfer(None, Some(&account), &amount)?;
            let mut new_balance = self._balance_of(&account);
            new_balance += amount;
            self.balances.insert(&account, &new_balance);
            self.supply += amount;
            self._after_token_transfer(None, Some(&account), &amount)?;
            self._emit_transfer_event(None, Some(account), amount);

            Ok(())
        }

        fn _burn_from(&mut self, account: AccountId, amount: u128) -> Result<()> {
            let mut from_balance = self._balance_of(&account);

            if from_balance < amount {
                return Err(PSP22Error::InsufficientBalance);
            }

            self._before_token_transfer(Some(&account), None, &amount)?;

            from_balance -= amount;
            self.balances.insert(&account, &from_balance);
            self.supply -= amount;
            self._after_token_transfer(Some(&account), None, &amount)?;
            self._emit_transfer_event(Some(account), None, amount);

            Ok(())
        }

        fn _before_token_transfer(
            &mut self,
            _from: Option<&AccountId>,
            _to: Option<&AccountId>,
            _amount: &u128,
        ) -> Result<()> {
            if self.paused() {
                return Err(PSP22Error::Custom(String::from("Paused")));
            }
            // only whitelisted accounts can transfer
            if self.is_required_whiteList == true {
                if !_from.is_none() && self.whitelist.get(_from.unwrap_or(&ZERO_ADDRESS.into())).unwrap_or(false) == false {
                    return Err(PSP22Error::Custom(String::from("From address is not whitelisted")));
                }
                if !_to.is_none() && *_to.unwrap() != self.owner.unwrap_or(ZERO_ADDRESS.into()) && self.whitelist.get(_to.unwrap_or(&ZERO_ADDRESS.into())).unwrap_or(false) == false {
                    return Err(PSP22Error::Custom(String::from("To address is not whitelisted")));
                }
            }

            // only non-blacklisted accounts can transfer
            if self.is_required_blackList == true {
                if !_from.is_none() && self.blacklist.get(_from.unwrap_or(&ZERO_ADDRESS.into())).unwrap_or(false) == true {
                    return Err(PSP22Error::Custom(String::from("From address is blacklisted")));
                }
                if !_to.is_none() && self.blacklist.get(_to.unwrap_or(&ZERO_ADDRESS.into())).unwrap_or(false) == true {
                    return Err(PSP22Error::Custom(String::from("To address is blacklisted")));
                }
            }

            let received_value = self.env().transferred_value();
            // check tax fee
            if received_value < self.tax_fee {
                if self.list_ignore_from_tax_fee.get(&self.env().caller()).unwrap_or(false) == true {
                    return Ok(());
                }
                return Err(PSP22Error::Custom(String::from("NotExactTaxFee")));
            }
            Ok(())
        }

        fn _after_token_transfer(&mut self, _from: Option<&AccountId>, _to: Option<&AccountId>, _amount: &u128) -> Result<()> {
            if self.is_require_max_alloc_per_address {
                if _to.is_none() || self.owner.unwrap_or(ZERO_ADDRESS.into()) == *_to.unwrap() {
                    return Ok(());
                }
                if self.balance_of(*_to.unwrap()) >= self.max_alloc_per_user {
                    return Err(PSP22Error::Custom(String::from("Exceeded max allocation per address")));
                }
            }
            Ok(())
        }

        fn _emit_transfer_event(
            &self,
            _from: Option<AccountId>,
            _to: Option<AccountId>,
            _amount: u128,
        ) {
            Token::emit_event(
                self.env(),
                Event::Transfer(Transfer {
                    from: _from,
                    to: _to,
                    value: _amount,
                }),
            )
        }

        fn _emit_approval_event(&self, _owner: AccountId, _spender: AccountId, _amount: u128) {
            Token::emit_event(
                self.env(),
                Event::Approval(Approval {
                    owner: _owner,
                    spender: _spender,
                    value: _amount,
                }),
            )
        }

        fn _require_owner(&self) -> Result<()> {
            if self.owner.is_none() || self.owner.unwrap() != self.env().caller() {
                return Err(PSP22Error::Custom("Not owner".to_string()));
            }
            Ok(())
        }
    }

    /// Event emitted when a token transfer occurs.
    #[ink(event)]
    #[derive(Debug)]
    pub struct Transfer {
        #[ink(topic)]
        pub from: Option<AccountId>,
        #[ink(topic)]
        pub to: Option<AccountId>,
        pub value: u128,
    }

    /// Event emitted when an approval occurs that `spender` is allowed to withdraw
    /// up to the amount of `value` tokens from `owner`.
    #[ink(event)]
    #[derive(Debug)]
    pub struct Approval {
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        spender: AccountId,
        value: u128,
    }
}
