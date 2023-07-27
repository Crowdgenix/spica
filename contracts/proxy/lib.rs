#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod proxy {
    use ink::env::call::Call;

    #[ink(storage)]
    pub struct Proxy {
        /// The `AccountId` of a contract where any call that does not match a
        /// selector of this contract is forwarded to.
        forward_to: AccountId,
        /// The `AccountId` of a privileged account that can update the
        /// forwarding address. This address is set to the account that
        /// instantiated this contract.
        admin: AccountId,
    }

    impl Proxy {
        #[ink(constructor)]
        pub fn new(forward_to: AccountId, admin: AccountId) -> Self {
            Self {
                forward_to,
                admin,
            }
        }
        /// Changes the `AccountId` of the contract where any call that does
        /// not match a selector of this contract is forwarded to.
        #[ink(message)]
        pub fn change_forward_address(&mut self, new_address: AccountId) {
            assert_eq!(
                self.env().caller(),
                self.admin,
                "caller {:?} does not have sufficient permissions, only {:?} does",
                self.env().caller(),
                self.admin,
            );
            self.forward_to = new_address;
        }

        /// Fallback message for a contract call that doesn't match any
        /// of the other message selectors.
        ///
        /// # Note:
        ///
        /// - We allow payable messages here and would forward any optionally supplied
        ///   value as well.
        /// - If the self receiver were `forward(&mut self)` here, this would not
        ///   have any effect whatsoever on the contract we forward to.
        #[ink(message, payable, selector = _)]
        pub fn forward(&self) -> u32 {
            ink::env::call::build_call::<ink::env::DefaultEnvironment>()
                .call_type(
                    Call::new(self.forward_to)
                        .transferred_value(self.env().transferred_value())
                        .gas_limit(0),
                )
                .call_flags(
                    ink::env::CallFlags::default()
                        .set_forward_input(true)
                        .set_tail_call(true),
                )
                .try_invoke()
                .unwrap_or_else(|err| {
                    panic!(
                        "cross-contract call to {:?} failed due to {:?}",
                        self.forward_to, err
                    )
                }).unwrap();
            unreachable!(
                "the forwarded call will never return since `tail_call` was set"
            );
        }
    }
}
