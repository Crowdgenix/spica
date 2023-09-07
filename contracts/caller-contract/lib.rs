#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![feature(min_specialization)]

#[ink::trait_definition]
pub trait Callee {
    #[ink(message)]
    fn emit_data(&mut self);
}

#[ink::contract]
pub mod caller {
    use crate::Callee;
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
    use callee::callee::{CalleeContractRef};


    type Event = <CallerContract as ContractEventBase>::Type;


    #[ink(event)]
    pub struct TestCallerTestEvent1Event1 {
        #[ink(topic)]
        pub ido_token: u128,
        pub price: u128,
        pub price_decimals: u32,
    }

    #[ink(event)]
    pub struct TestCallerTestEvent1Event2 {
        #[ink(topic)]
        pub buyer: u128,
        pub native_amount: u128,
        pub ido_token_amount: u128,
        pub nonce: u128,
    }

    #[ink(storage)]
    pub struct CallerContract {
        callee: AccountId,
    }

    impl CallerContract {
        /// constructor of IDO contract
        #[ink(constructor)]
        pub fn new(callee_: AccountId) -> Self {
            Self {
                callee: callee_,
            }
        }

        #[ink(message)]
        pub fn cross_call_01(&mut self) {
            let mut interface: CalleeContractRef = ink::env::call::FromAccountId::from_account_id(self.callee);
            interface.emit_data();

            Self::emit_event(Self::env(), Event::TestCallerTestEvent1Event1(TestCallerTestEvent1Event1 {
                ido_token: 0,
                price: 0,
                price_decimals: 0,
            }));
        }

        #[ink(message)]
        pub fn cross_call_02(&mut self) {
            let mut interface: CalleeContractRef = ink::env::call::FromAccountId::from_account_id(self.callee);
            interface.emit_data();
            Self::emit_event(Self::env(), Event::TestCallerTestEvent1Event1(TestCallerTestEvent1Event1 {
                ido_token: 0,
                price: 0,
                price_decimals: 0,
            }));

            Self::emit_event(Self::env(), Event::TestCallerTestEvent1Event2(TestCallerTestEvent1Event2 {
                buyer: 0,
                native_amount: 0,
                ido_token_amount: 0,
                nonce: 0,
            }));
        }

        fn emit_event<EE>(emitter: EE, event: Event)
            where
                EE: EmitEvent<CallerContract>,
        {
            emitter.emit_event(event);
        }
    }
}
