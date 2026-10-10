//! Token Wallet — a minimal fungible-token ledger as an Internet Computer
//! canister.
//!
//! Balances live in a `HashMap` inside one `Ledger` struct persisted to stable
//! memory, so the ledger survives canister upgrades. All ledger math lives in
//! [`ledger`] (pure Rust, unit-tested); this module is the thin canister
//! interface.

mod ledger;

use candid::Principal;
use ic_cdk::{init, post_upgrade, pre_upgrade, query, storage, update};
use ledger::{Ledger, TokenMetadata};

fn load_ledger() -> Ledger {
    storage::stable_restore::<(Ledger,)>()
        .unwrap_or_default()
        .0
}

fn save_ledger(ledger: &Ledger) {
    storage::stable_save((ledger,)).expect("failed to save ledger to stable memory");
}

/// Caller identity as text, used for owner checks.
fn caller_id() -> String {
    ic_cdk::api::msg_caller().to_text()
}

#[init]
fn init(name: String, symbol: String, decimals: u8, owner: Principal) {
    save_ledger(&Ledger::new(name, symbol, decimals, owner.to_text()));
}

#[pre_upgrade]
fn pre_upgrade() {
    // Ledger is already in stable memory; nothing extra to do.
}

#[post_upgrade]
fn post_upgrade() {
    // State is restored lazily from stable memory on first use.
}

/// Create new tokens. Only the owner may mint.
#[update]
fn mint(to: String, amount: u64) -> Result<(), String> {
    let mut ledger = load_ledger();
    let res = ledger
        .mint(&caller_id(), &to, amount)
        .map_err(|e| e.to_string());
    if res.is_ok() {
        save_ledger(&ledger);
    }
    res
}

/// Destroy tokens. The account holder or the owner may burn.
#[update]
fn burn(from: String, amount: u64) -> Result<(), String> {
    let mut ledger = load_ledger();
    let res = ledger
        .burn(&caller_id(), &from, amount)
        .map_err(|e| e.to_string());
    if res.is_ok() {
        save_ledger(&ledger);
    }
    res
}

/// Move tokens between accounts.
#[update]
fn transfer(from: String, to: String, amount: u64) -> Result<(), String> {
    let mut ledger = load_ledger();
    let res = ledger.transfer(&from, &to, amount).map_err(|e| e.to_string());
    if res.is_ok() {
        save_ledger(&ledger);
    }
    res
}

/// Balance of an account, 0 for unknown accounts.
#[query]
fn balance_of(account: String) -> u64 {
    load_ledger().balance_of(&account)
}

/// Total minted minus total burned.
#[query]
fn total_supply() -> u64 {
    load_ledger().total_supply
}

/// Token name, symbol, decimals, owner and supply.
#[query]
fn metadata() -> TokenMetadata {
    load_ledger().metadata()
}

// Export the Candid interface so `dfx` can generate `src/token_wallet.did`.
ic_cdk::export_candid!();

#[cfg(test)]
mod candid_gen {
    #[test]
    fn generate_did_file() {
        let did = super::__export_service();
        std::fs::write("src/token_wallet.did", did).expect("write did");
    }
}
