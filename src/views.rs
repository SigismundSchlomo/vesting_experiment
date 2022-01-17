use near_sdk::{near_bindgen};
use near_sdk::json_types::U128;

use crate::*;

#[near_bindgen]
impl Contract {

    pub fn get_balance(&self, account_id: AccountId) -> (U128, U128) {
        let (locked, claimed) = self.internal_get_balance(&account_id);
        (U128(locked), U128(claimed))
    }

}