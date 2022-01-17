use near_contract_standards::storage_management::{StorageBalance, StorageBalanceBounds, StorageManagement};
use near_sdk::{AccountId, assert_one_yocto, env, log, Promise, require};
use near_sdk::json_types::U128;

use crate::*;
use crate::account::Account;

#[near_bindgen]
impl StorageManagement for Contract {

    #[payable]
    fn storage_deposit(&mut self, account_id: Option<AccountId>, registration_only: Option<bool>) -> StorageBalance {
        let amount = env::attached_deposit();
        let account_id = account_id
            .unwrap_or_else(|| env::predecessor_account_id());
        let registration_only = registration_only.unwrap_or(false);
        let min_balance = self.storage_balance_bounds().min.0;
        let already_registered = self.accounts.contains_key(&account_id);
        if amount < min_balance && !already_registered {
            env::panic_str("ERR: deposit is less then min storage")
        }
        if registration_only {
            // Setups account but doesn't allocates space
            if already_registered {
                log!("ERR: account is already registered");
                if amount > 0 {
                    Promise::new(env::predecessor_account_id()).transfer(amount);
                }
            } else {
                self.internal_register_account(&account_id, min_balance);
                let refund = amount - min_balance;
                if refund > 0 {
                    Promise::new(env::predecessor_account_id()).transfer(refund);
                }
            }
        } else {
            self.internal_register_account(&account_id, amount);
        }
        self.storage_balance_of(account_id.try_into().unwrap())
            .unwrap()
    }

    #[allow(unused_variables)]
    #[payable]
    fn storage_withdraw(&mut self, amount: Option<U128>) -> StorageBalance {
        assert_one_yocto();
        let account_id = env::predecessor_account_id();
        let amount = env::attached_deposit();
        let withdraw_amount = self.internal_storage_withdraw(&account_id, amount);
        Promise::new(account_id.clone()).transfer(withdraw_amount);
        self.storage_balance_of(account_id).unwrap()
    }

    #[allow(unused_variables)]
    #[payable]
    fn storage_unregister(&mut self, force: Option<bool>) -> bool {
        assert_one_yocto();
        let account_id = env::predecessor_account_id();
        if let Some(account) = self.internal_get_account(&account_id){
            require!(account.is_empty(), "ERR: non zero balance");
            self.accounts.remove(&account_id);
            Promise::new(env::predecessor_account_id()).transfer(account.near_amount);
            true
        } else {
            false
        }
    }

    fn storage_balance_bounds(&self) -> StorageBalanceBounds {
        StorageBalanceBounds {
            min: Account::min_storage_usage().into(),
            max: None
        }
    }

    fn storage_balance_of(&self, account_id: AccountId) -> Option<StorageBalance> {
        self.internal_get_account(&account_id)
            .map(|account| {
                StorageBalance {
                    total: U128(account.near_amount),
                    available: U128(account.storage_available())
                }
            })
    }
}
