use candid::CandidType;
use ic_cdk::{init, query, storage, update};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

#[derive(CandidType, Deserialize, Serialize, Clone, Default)]
struct Account {
    id: String,
    balance: u64,
}

#[derive(CandidType, Deserialize, Serialize, Default)]
struct Ledger {
    accounts: HashMap<String, Account>,
    total_supply: u64,
}

#[init]
fn init() {
    let ledger = Ledger::default();
    storage::stable_save((ledger,)).unwrap();
}

fn load_ledger() -> Ledger {
    storage::stable_restore::<(Ledger,)>().unwrap_or_default().0
}

fn save_ledger(ledger: &Ledger) {
    storage::stable_save((ledger,)).unwrap();
}

#[update]
fn mint(account_id: String, amount: u64) {
    let mut ledger = load_ledger();
    let account = ledger.accounts.entry(account_id.clone()).or_insert(Account {
        id: account_id,
        balance: 0,
    });
    account.balance += amount;
    ledger.total_supply += amount;
    save_ledger(&ledger);
}

#[update]
fn transfer(from: String, to: String, amount: u64) -> Result<(), String> {
    let mut ledger = load_ledger();
    let from_account = ledger.accounts.get_mut(&from).ok_or("Sender account not found")?;
    if from_account.balance < amount {
        return Err("Insufficient balance".to_string());
    }
    from_account.balance -= amount;
    let to_account = ledger.accounts.entry(to.clone()).or_insert(Account {
        id: to,
        balance: 0,
    });
    to_account.balance += amount;
    save_ledger(&ledger);
    Ok(())
}

#[query]
fn balance_of(account_id: String) -> u64 {
    let ledger = load_ledger();
    ledger.accounts.get(&account_id).map_or(0, |account| account.balance)
}

#[query]
fn total_supply() -> u64 {
    let ledger = load_ledger();
    ledger.total_supply
}
