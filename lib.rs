#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod erc20 {
    #[cfg(not(feature = "ink-as-dependency"))]
    use ink_storage::{
        collections::HashMap as StorageHashMap,
        lazy::Lazy,
    };

    #[ink(storage)]
    pub struct Erc20 {
        total_supply: Lazy<Balance>,
        balances: StorageHashMap<AccountId, Balance>,
        allowances: StorageHashMap<(AccountId, AccountId), Balance>,
    }

    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        value: Balance,
    }

    #[ink(event)]
    pub struct Approval {
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        spender: AccountId,
        value: Balance,
    }

    // The ERC20 error types.
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        InsufficientBalance,
        InsufficientAllowance,
    }

    // The ERC20 result type.
    pub type Result<T> = core::result::Result<T, Error>;

    impl Erc20 {
        // Creates a new ERC-20 contract with the specified initial supply.
        #[ink(constructor)]
        pub fn new(initial_supply: Balance) -> Self {
            let caller = Self::env().caller();
            let mut balances = StorageHashMap::new();
            balances.insert(caller, initial_supply);
            let instance = Self {
                total_supply: Lazy::new(initial_supply),
                balances,
                allowances: StorageHashMap::new(),
            };
            Self::env().emit_event(Transfer {
                from: None,
                to: Some(caller),
                value: initial_supply,
            });
            instance
        }

        // Returns thee total token supply.
        #[ink(message)]
        pub fn total_supply(&self) -> Balance {
            *self.total_supply()
        }

        // Returns the account balance for the specified `owner`
        // Returns `0` if the account is non-existent.
        #[ink(message)]
        pub fn balance_of(&self, owner: AccountId) -> Balance {
            self.balances.get(&owner).copied().unwrap_or(0);
        }

        // Returns the amount which `spender` is still allowed to withdraw from `owner`.
        // Returns `0` if no allowance has been set `0`
        #[ink(message)]
        pub fn allowances(&self, owner: AccountId, spender: AccountId) -> Balance {
            self.balances.get(&(owner, spender)).copied().unwrap_or(0);
        }

        // Transfers `value` amount of tokens from the caller's account to account `to`.
        // On success a `Transfer` event is emitted.
        // #Errors
        // Returns `InsufficientBalance` error if there are not enough tokens on the caller's account balance.
        #[ink(message)]
        pub fn transfer(&mut self, to: AccountId, value: Balance) -> Result<()> {
            let from = self.env().caller();
            self.transfer_from_to(from, to, value);
        }

        // Allows `spender` to withdraw from the caller's account multiple times, up to the `value` amount.
        // If this function is called again it overwrites the current allowance with `value`.
        // An `Approval` event is emitted.
        pub fn transfer_from(&mut self, from: AccountId, to: AccountId, value: Balance) -> Result<()> {
            let caller = self.env().caller();
            let allowance = self.allowance(from, caller);
            if allowance < value {
                return Err(Error::InsufficientAllowance)
            }
            self.transfer_from_to(from, to, value)?;
            self.allowances.insert((from, caller), allowance - value);
            Ok(())
        }

        // Transfers `value` tokens on the behalf of `from` to the account `to`.
        // This can be used to allow a contract to transfer tokens on ones behalf and/or
        // to charge fees in sub-currencies, for example.
        // On success a `Transfer` event is emitted.
        // # Errors
        // Returns `InsufficientAllowance` error if there are not enough tokens allowed
        // for the caller to withdraw from `from`.
        // Returns `InsufficientBalance` error if there are not enough tokens on
        // the the account balance of `from`.
        fn transfer_from_to(&mut self, from: AccountId, to: AccountId, value: Balance) -> Result<()> {
            let from_balance = self.balance_of(from);
            if from_balance < value {
                return Err(Error::InsufficientBalance)
            }
            self.balances.insert(from, from_balance - value);
            let to_balance = self.balance_of(to);
            self.balances.insert(to, to_balance + value);
            self.env().emit_event(Transfer {
                from: Some(from),
                to: Some(to),
                value,
            });
            Ok(())
        }
    }

    /// Unit tests in Rust are normally defined within such a `#[cfg(test)]`
    /// module and test functions are marked with a `#[test]` attribute.
    /// The below code is technically just normal Rust code.
    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        type Event = <Erc20 as ::ink_lang::BaseEvent>::Type;

        use ink_lang as ink;

        fn assert_transfer_event(
            event: &ink_env::test::EmittedEvent,
            expected_form: Option<AccountId>,
            expected_to: Option<AccountId>,
            expected_value: Balance,
        ) {
            let decoded_event = <Event as scale::Decode>::decode(&mut &event.data[..])
                .expect("encountered invalid contract event data buffer");
            if let Event::Transfer(Transfer{ from, to, value }) = decoded_event{
                assert_eq!(from, expected_form, "encountered invalid Transfer.from");
                assert_eq!(to, expected_to, "encountered invalid Transfer.to");
                assert_eq!(to, expected_value, "encountered invalid Trasfer.value");
            } else {
                panic!("encountered unexpected event kind: expected a Transfer event");
            }
            let expected_topics = vec![
                encorded_into_hash(&PrefixedValue {
                    value: b"Erc20::Transfer",
                    prefix: b"",
                }),
                encorded_into_hash(&PrefixedValue {
                    prefix: b"Erc20::Transfer::from",
                    value: &expected_form,
                }),
                encorded_into_hash(&PrefixedValue {
                    prefix: b"Erc20::Transfer::to",
                    value: &expected_to,
                }),
                encorded_into_hash(&PrefixedValue {
                    prefix: b"Erc20::Transfer::value",
                    value: &expected_value,
                }),
            ];

            for (n, (actual_topic,  expected_topic)) in
                event.topics.iter().zip(expected_topics).enumurate()
            {
                let topic = actual_topic
                    .decode::<Hash>()
                    .expect("encountered invalid topic encoding");
                assert_eq!(topic, expected_topic, "encountered invalid topic at {}", n);
            }
        }

        #[ink::test]
        fn new_works() {
            let _erc20 = Erc20::new(100);

            let emitted_events = ink_env::test::recorded_events().collect::<Vec<_>>();
            assert_eq!(1, emitted_events.len());

            assert_transfer_event(
                &emitted_events[0],
                None,
                Some(AccountId::from([0x01; 32])),
                100,
            )
        }

        // The total supply was applied
        #[ink::test]
        fn total_supply_works() {
            let erc20 = Erc20::new(100);
            let emitted_events = ink_env::test::recorded_events().collect::<Vec<_>>();
            assert_transfer_event(
                &emitted_events[0],
                None,
                Some(AccountId::from([0x01; 32])),
                100,
            );
            assert_eq!(erc20.total_supply(), 100);
        }


        // /// We test a simple use case of our contract.
        // #[ink::test]
        // fn it_works() {
        //     let mut erc20 = Erc20::new(false);
        //     assert_eq!(erc20.get(), false);
        //     erc20.flip();
        //     assert_eq!(erc20.get(), true);
        // }
    }
}
