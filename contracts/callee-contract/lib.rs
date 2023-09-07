#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![feature(min_specialization)]

#[ink::contract]
pub mod callee {
    use ink::{
        env::{
            hash,
        },
        codegen::{
            EmitEvent,
            Env,
        },
        prelude::string::{String, ToString},
        reflect::ContractEventBase,
    };

    type Event = <CalleeContract as ContractEventBase>::Type;

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        /// Returned if not enough balance to fulfill a request is available.
        InsufficientBalance,
        /// Returned if not enough allowance to fulfill a request is available.
        InsufficientAllowance,
    }

    #[ink(event)]
    pub struct TestEvent1 {
        #[ink(topic)]
        pub ido_token: u128,
        pub price: u128,
        pub price_decimals: u32,
    }

    #[ink(event)]
    pub struct TestEvent2 {
        #[ink(topic)]
        pub buyer: u128,
        pub native_amount: u128,
        pub ido_token_amount: u128,
        pub nonce: u128,
    }

    #[ink(storage)]
    #[derive(Default)]
    pub struct CalleeContract {
        is_initialized: bool,
    }

    impl CalleeContract {
        /// constructor of IDO contract
        #[ink(constructor)]
        pub fn new() -> Self {
            let mut instance = Self::default();
            instance.is_initialized = false;
            instance
        }

        #[ink(message)]
        pub fn emit_data(&mut self) -> Result<(), Error> {
            Self::emit_event(Self::env(), Event::TestEvent1(TestEvent1 {
                ido_token: 0,
                price: 0,
                price_decimals: 0,
            }));

            Self::emit_event(Self::env(), Event::TestEvent2(TestEvent2 {
                buyer: 0,
                native_amount: 0,
                ido_token_amount: 0,
                nonce: 0,
            }));
            Ok(())
        }

        fn emit_event<EE>(emitter: EE, event: Event)
            where
                EE: EmitEvent<CalleeContract>,
        {
            emitter.emit_event(event);
        }
    }
}
