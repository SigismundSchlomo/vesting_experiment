use near_sdk::{AccountId, env, Gas, ext_contract};
use near_sdk::json_types::U128;

/// Amount of gas for fungible token transfers
pub const GAS_FOR_FT_TRANSFER: Gas = Gas(20_000_000_000_000);
pub const GAS_FOR_RESOLVE_TRANSFER: Gas = Gas(20_000_000_000_000);

//TODO: Add reasonable token id on official token account
pub const TOKEN_ID: &str = "dev-1642411965681-90797618493030";

pub fn validate_account_id(string: String) -> AccountId {
    if let Ok(account_id) = AccountId::try_from(string) {
        account_id
    } else {
        env::panic_str("ACCOUNT_ID_IS_INVALID")
    }
}

#[ext_contract(ext_self)]
pub trait ExtSelf {
    fn withdraw_callback(&mut self, sender_id: AccountId, amount: U128);
}