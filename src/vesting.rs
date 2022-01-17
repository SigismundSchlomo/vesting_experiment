use near_sdk::{env, log, near_bindgen, require};
use near_sdk::json_types::U128;

use crate::*;

const TOKEN_PRICE: u128 = 10u128.pow(24);

#[near_bindgen]
impl Contract {

    #[payable]
    pub fn buy(&mut self, amount: U128) {
        let sender_id = env::predecessor_account_id();
        let total_price = amount.0 * TOKEN_PRICE;
        require!(total_price == env::attached_deposit(), "ERR: deposit must be equal price  to be paid");
        log!("{} of token was bought", amount.0);
        self.internal_deposit(&sender_id, amount.0);
    }

    pub fn claim(&mut self) {
        let sender_id = env::predecessor_account_id();
        let (amount, _) = self.internal_get_balance(&sender_id);
        self.internal_claim(&sender_id, amount);
    }

}