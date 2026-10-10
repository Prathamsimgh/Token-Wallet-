//! Pure token-ledger logic. Kept free of `ic_cdk` so it can be unit-tested
//! on the host with `cargo test`.

use candid::CandidType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// A single token account.
#[derive(CandidType, Deserialize, Serialize, Clone, Default, Debug, PartialEq)]
pub struct Account {
    pub id: String,
    pub balance: u64,
}

/// Token metadata (ERC-20 style).
#[derive(CandidType, Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct TokenMetadata {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub owner: String,
    pub total_supply: u64,
}

/// The whole ledger. One struct is persisted to stable memory, so every
/// update rewrites it — fine for a demo token, not for production scale.
#[derive(CandidType, Deserialize, Serialize, Default, Debug)]
pub struct Ledger {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    /// Account id allowed to mint. Compared against the caller's principal text.
    pub owner: String,
    pub accounts: HashMap<String, Account>,
    pub total_supply: u64,
}

/// Everything that can go wrong in a ledger operation.
#[derive(Debug, Clone, PartialEq)]
pub enum LedgerError {
    Unauthorized { caller: String },
    AccountNotFound { account: String },
    InsufficientBalance { account: String, have: u64, want: u64 },
    ArithmeticOverflow,
}

impl fmt::Display for LedgerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LedgerError::Unauthorized { caller } => {
                write!(f, "Unauthorized: {caller} is not the token owner")
            }
            LedgerError::AccountNotFound { account } => {
                write!(f, "Account not found: {account}")
            }
            LedgerError::InsufficientBalance { account, have, want } => {
                write!(
                    f,
                    "Insufficient balance: {account} has {have}, tried to spend {want}"
                )
            }
            LedgerError::ArithmeticOverflow => write!(f, "Arithmetic overflow"),
        }
    }
}

impl Ledger {
    pub fn new(name: String, symbol: String, decimals: u8, owner: String) -> Self {
        Ledger {
            name,
            symbol,
            decimals,
            owner,
            accounts: HashMap::new(),
            total_supply: 0,
        }
    }

    pub fn metadata(&self) -> TokenMetadata {
        TokenMetadata {
            name: self.name.clone(),
            symbol: self.symbol.clone(),
            decimals: self.decimals,
            owner: self.owner.clone(),
            total_supply: self.total_supply,
        }
    }

    fn account_mut(&mut self, id: &str) -> &mut Account {
        self.accounts.entry(id.to_string()).or_insert(Account {
            id: id.to_string(),
            balance: 0,
        })
    }

    /// Create new tokens. Only the owner may mint.
    pub fn mint(
        &mut self,
        caller: &str,
        to: &str,
        amount: u64,
    ) -> Result<(), LedgerError> {
        if caller != self.owner {
            return Err(LedgerError::Unauthorized {
                caller: caller.to_string(),
            });
        }
        let account = self.account_mut(to);
        account.balance = account
            .balance
            .checked_add(amount)
            .ok_or(LedgerError::ArithmeticOverflow)?;
        self.total_supply = self
            .total_supply
            .checked_add(amount)
            .ok_or(LedgerError::ArithmeticOverflow)?;
        Ok(())
    }

    /// Destroy tokens. The account holder or the owner may burn.
    pub fn burn(
        &mut self,
        caller: &str,
        from: &str,
        amount: u64,
    ) -> Result<(), LedgerError> {
        if caller != from && caller != self.owner {
            return Err(LedgerError::Unauthorized {
                caller: caller.to_string(),
            });
        }
        let account = self
            .accounts
            .get_mut(from)
            .ok_or(LedgerError::AccountNotFound {
                account: from.to_string(),
            })?;
        if account.balance < amount {
            return Err(LedgerError::InsufficientBalance {
                account: from.to_string(),
                have: account.balance,
                want: amount,
            });
        }
        account.balance -= amount;
        self.total_supply -= amount;
        Ok(())
    }

    /// Move tokens between accounts.
    pub fn transfer(
        &mut self,
        from: &str,
        to: &str,
        amount: u64,
    ) -> Result<(), LedgerError> {
        if from == to {
            return Ok(());
        }
        let from_account =
            self.accounts
                .get_mut(from)
                .ok_or(LedgerError::AccountNotFound {
                    account: from.to_string(),
                })?;
        if from_account.balance < amount {
            return Err(LedgerError::InsufficientBalance {
                account: from.to_string(),
                have: from_account.balance,
                want: amount,
            });
        }
        from_account.balance -= amount;
        let to_account = self.account_mut(to);
        to_account.balance = to_account
            .balance
            .checked_add(amount)
            .ok_or(LedgerError::ArithmeticOverflow)?;
        Ok(())
    }

    /// Balance of an account, 0 for unknown accounts.
    pub fn balance_of(&self, account: &str) -> u64 {
        self.accounts.get(account).map_or(0, |a| a.balance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ledger() -> Ledger {
        Ledger::new(
            "Demo Token".to_string(),
            "DMT".to_string(),
            8,
            "owner-principal".to_string(),
        )
    }

    #[test]
    fn mint_only_by_owner() {
        let mut l = ledger();
        assert!(l.mint("owner-principal", "alice", 100).is_ok());
        assert_eq!(l.balance_of("alice"), 100);
        assert_eq!(l.total_supply, 100);

        let err = l.mint("mallory", "mallory", 1_000_000).unwrap_err();
        assert_eq!(
            err,
            LedgerError::Unauthorized {
                caller: "mallory".to_string()
            }
        );
        assert_eq!(l.total_supply, 100);
    }

    #[test]
    fn mint_overflow_is_rejected() {
        let mut l = ledger();
        l.mint("owner-principal", "alice", u64::MAX).unwrap();
        let err = l.mint("owner-principal", "alice", 1).unwrap_err();
        assert_eq!(err, LedgerError::ArithmeticOverflow);
    }

    #[test]
    fn transfer_happy_path() {
        let mut l = ledger();
        l.mint("owner-principal", "alice", 100).unwrap();
        l.transfer("alice", "bob", 40).unwrap();
        assert_eq!(l.balance_of("alice"), 60);
        assert_eq!(l.balance_of("bob"), 40);
        assert_eq!(l.total_supply, 100); // supply unchanged
    }

    #[test]
    fn transfer_errors() {
        let mut l = ledger();
        l.mint("owner-principal", "alice", 50).unwrap();

        assert_eq!(
            l.transfer("ghost", "bob", 10).unwrap_err(),
            LedgerError::AccountNotFound {
                account: "ghost".to_string()
            }
        );
        assert_eq!(
            l.transfer("alice", "bob", 51).unwrap_err(),
            LedgerError::InsufficientBalance {
                account: "alice".to_string(),
                have: 50,
                want: 51,
            }
        );
        // self-transfer is a no-op
        assert!(l.transfer("alice", "alice", 10).is_ok());
        assert_eq!(l.balance_of("alice"), 50);
    }

    #[test]
    fn burn_by_holder_and_owner() {
        let mut l = ledger();
        l.mint("owner-principal", "alice", 100).unwrap();

        l.burn("alice", "alice", 30).unwrap();
        assert_eq!(l.balance_of("alice"), 70);
        assert_eq!(l.total_supply, 70);

        // owner can burn from any account
        l.burn("owner-principal", "alice", 70).unwrap();
        assert_eq!(l.balance_of("alice"), 0);
        assert_eq!(l.total_supply, 0);

        // strangers cannot burn
        l.mint("owner-principal", "bob", 10).unwrap();
        assert!(l.burn("mallory", "bob", 1).is_err());
        assert_eq!(l.balance_of("bob"), 10);
    }

    #[test]
    fn unknown_accounts_read_zero() {
        let l = ledger();
        assert_eq!(l.balance_of("nobody"), 0);
    }

    #[test]
    fn metadata_roundtrip() {
        let l = ledger();
        let m = l.metadata();
        assert_eq!(m.name, "Demo Token");
        assert_eq!(m.symbol, "DMT");
        assert_eq!(m.decimals, 8);
        assert_eq!(m.owner, "owner-principal");
    }
}
