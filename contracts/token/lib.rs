#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![feature(min_specialization)]
#![allow(clippy::let_unit_value)]

pub use self::token::*;

#[openbrush::contract]
pub mod token {
    use openbrush::contracts::psp22::{Data, Internal, PSP22Error};
    use openbrush::traits::{Storage, String, ZERO_ADDRESS};

    use ink::{
        codegen::{EmitEvent, Env},
        prelude::vec::Vec,
        reflect::ContractEventBase,
        storage::Mapping,
        env::transfer,
    };

    use logics::traits::token::{self, PSP22};

    use openbrush::{
        contracts::{
            ownable::*,
            pausable::*,
            psp22::{
                self,
                *,
                extensions::{burnable, metadata, mintable},
            },
        },
        modifiers,
    };

    /// Result type
    pub type Result<T> = core::result::Result<T, PSP22Error>;

    /// Event type
    pub type Event = <Token as ContractEventBase>::Type;

    pub(super) mod selectors {
        // Selectors for the methods of interest on PSP22.
        // NOTE: They can be found in `target/ink/metadata.json` after building the contract.
        pub const TOTAL_SUPPLY_SELECTOR: [u8; 4] = [0x16, 0x2d, 0xf8, 0xc2];
        pub const TRANSFER_TO_SELECTOR: [u8; 4] = [0xdb, 0x20, 0xf9, 0xf5];
        pub const TRANSFER_FROM_SELECTOR: [u8; 4] = [0x54, 0xb3, 0xc7, 0x6e];
        pub const BALANCE_OF_SELECTOR: [u8; 4] = [0x65, 0x68, 0x38, 0x2f];
        pub const APPROVE_ALLOWANCE_SELECTOR: [u8; 4] = [0xb2, 0x0f, 0x1b, 0xbd];
        pub const INCREASE_ALLOWANCE_SELECTOR: [u8; 4] = [0x96, 0xd6, 0xb5, 0x7a];
        pub const MINT_SELECTOR: [u8; 4] = [0xfc, 0x3c, 0x75, 0xd4];
        pub const BURN_SELECTOR: [u8; 4] = [0x7a, 0x9d, 0xa5, 0x10];
    }

    #[ink(storage)]
    #[derive(Default, Storage)]
    pub struct Token {
        #[storage_field]
        psp22: psp22::Data,
        #[storage_field]
        metadata: metadata::Data,
        #[storage_field]
        ownable: ownable::Data,
        #[storage_field]
        pausable: pausable::Data,
        is_required_whiteList: bool,
        is_required_blackList: bool,
        is_burnable: bool,
        is_mintable: bool,
        is_pausable: bool,
        is_require_max_alloc_per_address: bool,
        max_alloc_per_user: u128,
        is_force_transfer_enable: bool,
        whitelist: Mapping<AccountId, bool>,
        blacklist: Mapping<AccountId, bool>,
        tax_fee: Balance,
        tax_fee_receiver: Option<AccountId>,
        document: String,
    }


    impl psp22::Transfer for Token {
        #[modifiers(when_not_paused)]
        fn _before_token_transfer(
            &mut self,
            _from: Option<&AccountId>,
            _to: Option<&AccountId>,
            _amount: &Balance,
        ) -> Result<()> {
            // if enabled whitelist and caller is not whitelisted or recipient is not whitelisted
            if self.is_required_whiteList == true {
                if self.whitelist.get(_from.unwrap()).unwrap_or(false) == false {
                    return Err(PSP22Error::Custom(String::from("Caller is not whitelisted")));
                }
                if self.whitelist.get(_to.unwrap()).unwrap_or(false) == false {
                    return Err(PSP22Error::Custom(String::from("Recipient is not whitelisted")));
                }
            }

            if self.is_required_blackList == true {
                if self.blacklist.get(_from.unwrap()).unwrap_or(false) == true {
                    return Err(PSP22Error::Custom(String::from("Caller is blacklisted")));
                }
                if self.blacklist.get(_to.unwrap()).unwrap_or(false) == true {
                    return Err(PSP22Error::Custom(String::from("Recipient is blacklisted")));
                }
            }

            let to_addr = _to.unwrap();
            if self.is_require_max_alloc_per_address {
                if (self.ownable.owner != *to_addr || _to.is_none()) && self.balance_of(*to_addr) >= self.max_alloc_per_user {

                }
            }

            let received_value = self.env().transferred_value();
            if received_value < self.tax_fee {
                return Err(PSP22Error::Custom(String::from("NotExactTaxFee")));
            }
            Ok(())
        }
    }


    impl logics::traits::token::PSP22 for Token {}

    impl Token {
        #[ink(constructor)]
        pub fn new(owner: AccountId, name: String, symbol: String, decimals: u8, total_supply: Balance, is_require_whitelist: bool,
                   is_require_blacklist: bool, is_burnable: bool, is_mintable: bool, is_force_transfer_enable: bool,
                   is_pausable: bool, is_require_max_alloc_per_address: bool, max_alloc_per_user: u128, tax_fee_receiver: AccountId, tax_fee: u128, document: String) -> Self {
            let mut instance = Self::default();

            instance._init_with_owner(owner);
            instance.metadata.name = Some(name);
            instance.metadata.symbol = Some(symbol);
            instance.metadata.decimals = decimals;
            instance.whitelist = Mapping::default();
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
            instance
                .psp22
                ._mint_to(owner, total_supply)
                .unwrap();

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
        pub fn tax_fee(&self) -> Balance {
            self.tax_fee
        }

        #[ink(message)]
        #[modifiers(only_owner)]
        pub fn change_pause_state(&mut self) -> Result<()> {
            if !self.is_pausable {
                return Err(PSP22Error::Custom(String::from("not pausable")));
            }
            if self.paused() {
                self._unpause()
            } else {
                self._pause()
            }
        }

        #[ink(message)]
        #[modifiers(only_owner)]
        pub fn add_whitelist(&mut self, users: Vec<AccountId>) -> Result<()> {
            for user in users {
                self.whitelist.insert(user, &true);
            }
            Ok(())
        }

        #[ink(message)]
        #[modifiers(only_owner)]
        pub fn remove_whitelist(&mut self, users: Vec<AccountId>) -> Result<()> {
            for user in users {
                self.whitelist.insert(user, &false);
            }
            Ok(())
        }


        #[ink(message)]
        #[modifiers(only_owner)]
        pub fn add_blacklist(&mut self, users: Vec<AccountId>) -> Result<()> {
            for user in users {
                self.blacklist.insert(user, &true);
            }
            Ok(())
        }

        #[ink(message)]
        #[modifiers(only_owner)]
        pub fn remove_blacklist(&mut self, users: Vec<AccountId>) -> Result<()> {
            for user in users {
                self.blacklist.insert(user, &false);
            }
            Ok(())
        }

        #[ink(message)]
        #[modifiers(only_owner)]
        /// Mints the `amount` of underlying tokens to the recipient identified by the `account` address.
        pub fn force_transfer(&mut self, from_account: AccountId, to_account: AccountId, amount: Balance) -> Result<()> {
            if !self.is_force_transfer_enable {
                return Err(PSP22Error::Custom(String::from("not allow force transfer")));
            }
            self._transfer_from_to(from_account, to_account, amount, Vec::new())?;
            Ok(())
        }

        #[ink(message)]
        /// Mints the `amount` of underlying tokens to the recipient identified by the `account` address.
        pub fn claim_tax_fee(&mut self, to: AccountId, amount: Balance) -> Result<()> {
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
        pub fn burn(&mut self, amount: Balance) -> Result<()> {
            if !self.is_burnable {
                return Err(PSP22Error::Custom(String::from("not burnable")));
            }
            self._burn_from(self.env().caller(), amount)
        }

        #[ink(message)]
        #[modifiers(only_owner)]
        pub fn set_code(&mut self, code_hash: [u8; 32]) -> Result<()> {
            ink::env::set_code_hash(&code_hash).unwrap_or_else(|err| {
                panic!(
                    "Failed to `set_code_hash` to {:?} due to {:?}",
                    code_hash, err
                )
            });
            ink::env::debug_println!("Switched code hash to {:?}.", code_hash);
            Ok(())
        }
    }

    // We have to implement the "main trait" for our contract to have the PSP22 methods available.
    // impl PSP22 for Token {}

    // And `PSP22Metadata` to get metadata-related methods.
    impl metadata::PSP22Metadata for Token {}

    // And so on...
    impl Ownable for Token {}

    impl Pausable for Token {}

    impl mintable::PSP22Mintable for Token {
        #[ink(message)]
        #[modifiers(only_owner)]
        /// Mints the `amount` of underlying tokens to the recipient identified by the `account` address.
        fn mint(&mut self, account: AccountId, amount: Balance) -> Result<()> {
            if !self.is_mintable {
                return Err(PSP22Error::Custom(String::from("not mintable")));
            }
            self._mint_to(account, amount)
        }
    }

    // impl burnable::PSP22Burnable for Token {
    //     #[ink(message)]
    //     #[modifiers(only_owner)]
    //     /// Burns the `amount` of underlying tokens from the balance of `account` recipient.
    //     fn burn(&mut self, account: AccountId, amount: Balance) -> Result<()> {
    //         self._burn_from(account, amount)
    //     }
    // }

    // Overwrite the `psp22::Internal` trait to emit the events as described in the PSP22 spec:
    // https://github.com/w3f/PSPs/blob/master/PSPs/psp-22.md#transfer
    impl psp22::Internal for Token {
        fn _emit_transfer_event(
            &self,
            _from: Option<AccountId>,
            _to: Option<AccountId>,
            _amount: Balance,
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

        fn _emit_approval_event(&self, _owner: AccountId, _spender: AccountId, _amount: Balance) {
            Token::emit_event(
                self.env(),
                Event::Approval(Approval {
                    owner: _owner,
                    spender: _spender,
                    value: _amount,
                }),
            )
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
        pub value: Balance,
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
        value: Balance,
    }
}
