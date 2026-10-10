# Token Wallet

A minimal fungible-token ledger as an Internet Computer canister. Balances
live in a `HashMap` persisted to stable memory, so the ledger survives
canister upgrades.

## Build

Needs `candid`, `ic-cdk`, and `serde` (see `lib.rs` imports). This is a
canister crate — build it for `wasm32-unknown-unknown` and deploy with
`dfx`:

```sh
rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown --release
```

## API

| Method | Kind | Args | Returns |
|--------|------|------|---------|
| `mint` | update | `account_id: Text`, `amount: Nat64` | — |
| `transfer` | update | `from: Text`, `to: Text`, `amount: Nat64` | `Result<(), Text>` |
| `balance_of` | query | `account_id: Text` | `Nat64` |
| `total_supply` | query | — | `Nat64` |

- `mint` creates the account on first use and grows `total_supply`.
- `transfer` fails with `"Sender account not found"` if `from` does not
  exist, and with `"Insufficient balance"` if the balance is too low.
- `balance_of` returns `0` for unknown accounts instead of failing.

## Notes

- Single-ledger design: one `Ledger` struct holds all accounts, so every
  update rewrites the whole ledger to stable storage. Fine for a demo;
  a production token would shard or index by account.
- `u64` balances are unchecked on addition in `mint` — debug builds panic
  on overflow, release builds wrap. Worth a `checked_add` if this ever
  leaves the prototype stage.
