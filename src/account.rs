use near_contract_standards::fungible_token::core_impl::ext_fungible_token;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::json_types::U128;
use near_sdk::{
    assert_one_yocto, env, log, require, AccountId, Balance, Gas, Promise, PromiseOrValue,
    PromiseResult, StorageUsage,
};

use crate::utils::{
    ext_self, validate_account_id, GAS_FOR_FT_TRANSFER, GAS_FOR_RESOLVE_TRANSFER, TOKEN_ID,
};
use crate::*;

const U128_STORAGE: StorageUsage = 16;
const U64_STORAGE: StorageUsage = 8;
const U32_STORAGE: StorageUsage = 4;

// U128_STORAGE: near_amount storage
// + U128_STORAGE: amount_locked storage
// + U128_STORAGE: amount_claimed storage
// + U64_STORAGE: storage_used
pub const INIT_ACCOUNT_STORAGE: StorageUsage =
    U128_STORAGE + U128_STORAGE + U128_STORAGE + U64_STORAGE;

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Account {
    ///Native near used for storage
    pub near_amount: Balance,
    pub amount_locked: Balance,
    pub amount_claimed: Balance,
    pub storage_used: StorageUsage,
}

impl Account {
    pub fn new() -> Self {
        Self {
            near_amount: 0,
            amount_locked: 0,
            amount_claimed: 0,
            storage_used: 0,
        }
    }

    /// Deposits given amount of token to amount_locked balance
    pub(crate) fn deposit(&mut self, amount: Balance) {
        if let Some(result) = self.amount_locked.checked_add(amount) {
            self.amount_locked = result;
        } else {
            env::panic_str("ERR: Integer overflow occurred");
        }
    }

    /// Withdraws given amount of token of amount_claimed
    pub(crate) fn withdraw(&mut self, amount: Balance) {
        if let Some(result) = self.amount_claimed.checked_sub(amount) {
            self.amount_claimed = result;
        } else {
            env::panic_str("ERR: Not enough tokens claimed");
        }
    }

    /// Moves given amount from amount_locked to amount_claimed
    pub(crate) fn claim(&mut self, amount: Balance) {
        if let Some(locked_result) = self.amount_locked.checked_sub(amount) {
            if let Some(claimed_result) = self.amount_claimed.checked_add(amount) {
                self.amount_claimed = claimed_result;
                self.amount_locked = locked_result;
            }
            log!("{} tokens was claimed", amount);
        } else {
            env::panic_str("ERR: Not enough tokens locked")
        }
    }

    /// Returns true if account balance is equal to zero
    pub fn is_empty(&self) -> bool {
        self.amount_claimed == 0  && self.amount_locked == 0
    }

    /// Returns amount of near necessary to cover storage used by this data structure.
    pub fn storage_usage(&self) -> Balance {
        INIT_ACCOUNT_STORAGE as u128 * env::storage_byte_cost()
    }

    /// Returns amount of near available for storage
    pub fn storage_available(&self) -> Balance {
        if let Some(result) = self.near_amount.checked_sub(self.storage_usage()) {
            result
        } else {
            0
        }
    }

    //TODO: Add abstraction to return friendly map of balances
    /// Return tuple of locked and claimed amount
    pub fn get_balance(&self) -> (Balance, Balance) {
        (self.amount_locked, self.amount_claimed)
    }

    /// Asserts there is sufficient amount of near to cover storage usage.
    pub fn assert_storage_usage(&self) {
        require!(
            self.storage_usage() <= self.near_amount,
            "ERR: not enough deposit to cover storage usage"
        );
    }

    /// Returns minimal required account deposit
    pub fn min_storage_usage() -> Balance {
        INIT_ACCOUNT_STORAGE as Balance * env::storage_byte_cost()
    }

    /// Deposits to claim. Should only be use to refund after failed operation
    pub(self) fn deposit_to_claim(&mut self, amount: Balance) {
        if let Some(result) = self.amount_claimed.checked_add(amount) {
            self.amount_claimed = result
        } else {
            env::panic_str("ERR: integer overflow")
        }
    }
}

#[near_bindgen]
impl Contract {
    #[payable]
    pub fn withdraw(&mut self, amount: U128) -> Promise {
        assert_one_yocto();
        require!(amount.0 > 0, "ERR: illegal withdraw amount");
        let sender_id = env::predecessor_account_id();
        let mut account = self.internal_unwrap_account(&sender_id);
        account.withdraw(amount.0);
        self.internal_save_account(&sender_id, account);
        self.internal_send_tokens(&sender_id, amount.0)
    }

    #[private]
    pub fn withdraw_callback(&mut self, sender_id: AccountId, amount: U128) {
        require!(
            env::promise_results_count() == 1,
            "ERR: 1 promise result expected"
        );
        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(_) => {}
            PromiseResult::Failed => {
                if let Some(mut account) = self.internal_get_account(&sender_id) {
                    if account.storage_usage() <= account.near_amount {
                        account.deposit_to_claim(amount.0);
                        self.accounts.insert(&sender_id, &account);
                    } else {
                        log!(
                            "Account {} has not enough storage. Depositing to owner",
                            sender_id
                        );
                        self.internal_lost_and_found(amount.0);
                    }
                } else {
                    log!(
                        "Account {} is not registered. Depositing to owner",
                        sender_id
                    );
                    self.internal_lost_and_found(amount.0);
                }
            }
        }
    }
}

impl Contract {
    /// Registers account with given amount of near
    /// If account already registered adds amount to it's deposit
    pub fn internal_register_account(&mut self, account_id: &AccountId, amount: Balance) {
        let mut account = self.internal_unwrap_or_default_account(&account_id);
        account.near_amount += amount;
        self.internal_save_account(account_id, account);
    }

    /// Deposits given amount to the account and saves account
    pub fn internal_deposit(&mut self, sender_id: &AccountId, amount: Balance) {
        let mut account = self.internal_unwrap_account(sender_id);
        account.deposit(amount);
        self.internal_save_account(&sender_id, account);
    }

    /// Claims given amount to the account and saves account
    pub fn internal_claim(&mut self, sender_id: &AccountId, amount: Balance) {
        let mut account = self.internal_unwrap_account(sender_id);
        account.claim(amount);
        self.internal_save_account(&sender_id, account);
    }

    /// Withdraws given amount of near and saves account. Checks if operation is legal
    pub fn internal_storage_withdraw(&mut self, account_id: &AccountId, amount: Balance) -> u128 {
        let mut account = self.internal_unwrap_account(account_id);
        let available = account.storage_available();
        require!(available > 0, "Err: illegal withdraw amount");
        let mut withdraw_amount = amount;
        if amount == 0 {
            withdraw_amount = available;
        }
        require!(
            withdraw_amount <= available,
            "ERR: not enough tokens on deposit"
        );
        account.near_amount -= withdraw_amount;
        self.internal_save_account(account_id, account);
        withdraw_amount
    }

    pub fn internal_get_account(&self, account_id: &AccountId) -> Option<Account> {
        self.accounts.get(account_id)
    }

    pub fn internal_unwrap_account(&self, account_id: &AccountId) -> Account {
        self.internal_get_account(account_id)
            .expect("ERR: account is not registered")
    }

    pub fn internal_unwrap_or_default_account(&self, account_id: &AccountId) -> Account {
        self.internal_get_account(account_id)
            .unwrap_or_else(|| Account::new())
    }

    /// Checks if account have enough storage deposit and saves it into collection.
    pub fn internal_save_account(&mut self, account_id: &AccountId, account: Account) {
        account.assert_storage_usage();
        self.accounts.insert(&account_id, &account);
    }

    /// Returns balance of given account. First is locked and second is claimed
    pub fn internal_get_balance(&self, account_id: &AccountId) -> (Balance, Balance) {
        self.internal_unwrap_account(account_id).get_balance()
    }

    /// Deposits given amount to the owners address
    fn internal_lost_and_found(&mut self, amount: Balance) {
        let mut lost_and_found = self.internal_unwrap_or_default_account(&self.owner_id);
        lost_and_found.deposit_to_claim(amount);
        self.accounts.insert(&self.owner_id, &lost_and_found.into());
    }

    /// Sends given amount of tokens to given account
    fn internal_send_tokens(&mut self, sender_id: &AccountId, amount: Balance) -> Promise {
        ext_fungible_token::ft_transfer(
            sender_id.clone(),
            U128(amount),
            None,
            validate_account_id(TOKEN_ID.to_string()),
            1,
            GAS_FOR_FT_TRANSFER,
        ).then(ext_self::withdraw_callback(
            sender_id.clone(),
            U128(amount),
            env::current_account_id(),
            0,
            GAS_FOR_RESOLVE_TRANSFER,
        ))
    }
}
