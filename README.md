# Token Wallet

![Rust](https://img.shields.io/badge/Rust-1.99-000000?style=flat-square&logo=rust&logoColor=white)
![Internet Computer](https://img.shields.io/badge/Internet_Computer-Canister-6B4EFF?style=flat-square)
![License: MIT](https://img.shields.io/badge/License-MIT-green?style=flat-square)

A minimal fungible-token ledger as an Internet Computer canister, written in Rust.

Balances live in a `HashMap` inside one `Ledger` struct persisted to stable
memory, so the ledger survives canister upgrades. Minting is restricted to the
owner principal recorded at deploy time.

## Project structure

```
Token-Wallet-/
├── Cargo.toml          # canister crate (cdylib): candid, ic-cdk, serde
├── dfx.json            # canister definition for `dfx deploy`
├── src/
│   ├── lib.rs          # canister interface (init/update/query entry points)
│   ├── ledger.rs       # pure ledger logic + unit tests (no ic_cdk here)
│   └── token_wallet.did# Candid interface, extracted from the built wasm
├── LICENSE
└── README.md
```

All ledger math lives in `src/ledger.rs` with no `ic_cdk` dependency, so the
rules are unit-testable on the host with `cargo test`. `src/lib.rs` is the
thin canister glue: stable-memory load/save plus the `init`/`update`/`query`
entry points.

## Build & test

```sh
# unit tests (ledger logic, runs on the host)
cargo test

# build the canister wasm
rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown --release
# -> target/wasm32-unknown-unknown/release/token_wallet.wasm
```

## Deploy

```sh
dfx start --background
dfx deploy token_wallet --argument '( "Demo Token", "DMT", 8, principal "<OWNER_PRINCIPAL>" )'
```

## API

| Method | Kind | Args | Returns |
|--------|------|------|---------|
| `init` | init | `name: Text`, `symbol: Text`, `decimals: Nat8`, `owner: Principal` | — |
| `mint` | update | `to: Text`, `amount: Nat64` | `Result<(), Text>` — owner only |
| `burn` | update | `from: Text`, `amount: Nat64` | `Result<(), Text>` — holder or owner |
| `transfer` | update | `from: Text`, `to: Text`, `amount: Nat64` | `Result<(), Text>` |
| `balance_of` | query | `account: Text` | `Nat64` (0 for unknown accounts) |
| `total_supply` | query | — | `Nat64` |
| `metadata` | query | — | `TokenMetadata` |

Error cases use descriptive messages: `"Unauthorized: … is not the token owner"`,
`"Account not found: …"`, `"Insufficient balance: … has …, tried to spend …"`,
`"Arithmetic overflow"`.

## Notes

- Single-ledger design: one `Ledger` struct holds all accounts, so every update
  rewrites the whole ledger to stable storage. Fine for a demo; a production
  token would shard or index by account (see `ic-stable-structures`).
- All balance arithmetic uses `checked_add`; overflow is rejected instead of
  wrapping.
- `transfer` to self is a no-op.
