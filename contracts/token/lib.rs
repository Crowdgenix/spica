#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![feature(min_specialization)]
#![allow(clippy::let_unit_value)]

// pub use crate::token::selectors::*;
pub use self::token::*;
//
// #[modifier_definition]
// pub fn only_whitelisted<T, F, R, E>(instance: &mut T, body: F) -> Result<R, E>
//     where
//         T: Storage<token::Token>,
//         F: FnOnce(&mut T) -> Result<R, E>,
//         E: From<PSP22Error>,
// {
//     if instance.data().is_required_whiteList == true && instance.data(). != T::env().caller() {
//         return Err(From::from(FactoryError::CallerIsNotFeeSetter))
//     }
//     body(instance)
// }

#[openbrush::contract]
pub mod token {
    use ink::{
        codegen::{EmitEvent, Env},
        prelude::string::String,
        prelude::vec::Vec,
        reflect::ContractEventBase,
        storage::Mapping,
        env::transfer,
    };

    use openbrush::{
        contracts::{
            ownable::*,
            pausable::*,
            psp22::{
                self,
                *,
                extensions::{burnable, metadata, mintable},
                psp22::Internal,
                PSP22Error,
            },
        },
        modifiers,
        traits::Storage,
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
        whitelist: Mapping<AccountId, bool>,
        tax_fee: Balance,
        document: String,
    }

    impl psp22::Transfer for Token {
        #[modifiers(when_not_paused)]
        fn _before_token_transfer(
            &mut self,
            _from: Option<&AccountId>,
            _to: Option<&AccountId>,
            _amount: &Balance
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
            let received_value = self.env().transferred_value();
            if received_value < self.tax_fee {
                return Err(PSP22Error::Custom(String::from("NotExactTaxFee")));
            }
            Ok(())
        }
    }

    impl Token {
        #[ink(constructor)]
        pub fn new(owner: AccountId, name: String, symbol: String, decimals: u8, total_supply: Balance, is_require_whitelist: bool, tax_fee: u128, document: String) -> Self {
            let mut instance = Self::default();

            instance._init_with_owner(owner);
            instance.metadata.name = Some(name);
            instance.metadata.symbol = Some(symbol);
            instance.metadata.decimals = decimals;
            instance.whitelist = Mapping::default();
            instance.is_required_whiteList = is_require_whitelist;
            instance.tax_fee = tax_fee;
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
        /// Mints the `amount` of underlying tokens to the recipient identified by the `account` address.
        pub fn force_transfer(&mut self, from_account: AccountId, to_account: AccountId, amount: Balance) -> Result<()> {
            self._transfer_from_to(from_account, to_account, amount, Vec::new())?;
            Ok(())
        }

        #[ink(message)]
        #[modifiers(only_owner)]
        /// Mints the `amount` of underlying tokens to the recipient identified by the `account` address.
        pub fn claim_tax_fee(&mut self, to: AccountId, amount: Balance) -> Result<()> {
            let ok = self.env().transfer(to, amount);
            if ok.is_err() {
                return Err(PSP22Error::Custom(String::from("Error while transfer native")));
            }
            Ok(())
        }
    }

    // We have to implement the "main trait" for our contract to have the PSP22 methods available.
    impl PSP22 for Token {}

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
            self._mint_to(account, amount)
        }
    }

    impl burnable::PSP22Burnable for Token {
        #[ink(message)]
        #[modifiers(only_owner)]
        /// Burns the `amount` of underlying tokens from the balance of `account` recipient.
        fn burn(&mut self, account: AccountId, amount: Balance) -> Result<()> {
            self._burn_from(account, amount)
        }
    }

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
