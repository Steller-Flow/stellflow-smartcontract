# `escrow/` — Cargo workspace

This directory is the Cargo workspace for the StellFlow escrow contract. The
only member is `contracts/escrow/` (crate `stellflow-escrow`); the shared
`soroban-sdk` version and the size-optimised `release` profile live in the
workspace `Cargo.toml` here.

For what the contract does, its API, and the security model, see the
[root README](../README.md).

## Layout

```text
escrow/
├── Cargo.toml              # workspace: members, soroban-sdk version, release profile
└── contracts/escrow/
    ├── Cargo.toml          # crate: cdylib + rlib, `testutils` feature
    ├── src/
    │   ├── lib.rs          # #![no_std] crate root, module wiring, re-exports
    │   ├── contract.rs     # EscrowContract: the 32 exported contract functions
    │   ├── storage.rs      # persistent/instance storage accessors, TTL, roles
    │   ├── events.rs       # event symbols and emit_* helpers
    │   ├── types.rs        # Escrow, Milestone, EscrowEvent, status enums, DataKey
    │   ├── errors.rs       # EscrowError (contracterror) variants
    │   └── testutils.rs    # cfg(test) helpers (test env, funded escrow)
    └── tests/              # 9 integration suites, 108 tests
```

## Build

Soroban contracts must be built with `stellar contract build`, not
`cargo build` (see the [soroban-sdk docs](https://docs.rs/soroban-sdk)).
It targets `wasm32v1-none` and post-processes the binary.

```bash
# one-time setup
rustup target add wasm32v1-none
cargo install --locked stellar-cli   # or: brew install stellar-cli

# from this directory
stellar contract build
# → target/wasm32v1-none/release/stellflow_escrow.wasm
```

## Test and lint

```bash
# from this directory
cargo test --all
cargo test --all --features testutils
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
```

These are the same commands CI runs (`.github/workflows/ci.yml`).
