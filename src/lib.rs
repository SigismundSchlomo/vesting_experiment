use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{AccountId, near_bindgen, PanicOnDefault, BorshStorageKey};
use near_sdk::collections::{LookupMap, UnorderedSet};
use crate::account::Account;

mod account;
mod storage_impl;
mod utils;

#[derive(BorshSerialize, BorshStorageKey)]
pub(crate) enum StorageKey {
    Accounts,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
struct Contract {
    owner_id: AccountId,
    accounts: LookupMap<AccountId, Account>,
}

#[near_bindgen]
impl Contract {

    #[init]
    pub fn new(owner_id: AccountId) -> Self {
        Self {
            owner_id,
            accounts: LookupMap::new(StorageKey::Accounts),
        }
    }
}

#[cfg(test)]
mod tests {

}
