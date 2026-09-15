# Contributing to StellFlow

Thanks for your interest. This document covers how to set up the project,
the workflow we use, and what we expect from a pull request.

## Prerequisites

- **Rust stable** via [rustup](https://rustup.rs) (`rustup default stable`)
- The **`wasm32v1-none`** target: `rustup target add wasm32v1-none`
- **stellar-cli** ≥ 26: `cargo install --locked stellar-cli` or
  `brew install stellar-cli` — see the
  [installation guide](https://developers.stellar.org/docs/tools/cli/install-cli)

## Clone and test

```bash
git clone https://github.com/Steller-Flow/stellflow-smartcontract.git
cd stellflow-smartcontract/escrow
cargo test --all
```

You should see 108 tests pass across the 9 suites in
`escrow/contracts/escrow/tests/`.

## Workflow

1. **Comment on the issue first.** Say you'd like to take it and, for anything
   non-trivial, outline your approach. This avoids duplicate work and lets us
   catch design problems early. If there's no issue for what you want to
   change, open one.
2. **Branch** from `main`: `git checkout -b fix/short-description` or
   `feat/short-description`.
3. **Make your change** with focused commits and descriptive messages.
4. **Run the local checks** (below) until they all pass.
5. **Open a pull request** against `main`. Put `Closes #N` in the description
   so the issue is linked and closed on merge.

## Local checks

Run these from the `escrow/` directory before opening a PR. CI runs the same
commands and will reject a PR that fails any of them.

```bash
cd escrow
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test --all
stellar contract build
```

`stellar contract build` is the supported way to produce the WASM; a plain
`cargo build` compiles a native library, not a deployable contract.

## Pull request expectations

- **One issue per PR.** Keep unrelated changes out; open a second PR instead.
- **New behaviour needs a test.** Add it to the relevant suite under
  `escrow/contracts/escrow/tests/` (or a new `*_tests.rs` file if none fits).
- **Match the existing test style.** Each suite has a `setup()` that creates
  an `Env` with `mock_all_auths()`, generates addresses, and registers a
  Stellar Asset Contract as the test token, plus a `contract()` helper that
  registers `EscrowContract` and returns its client. Tests are named
  `test_<function>_<scenario>` and use `c.try_<fn>(...)` to assert error
  cases.
- **Don't touch contract logic in a docs or CI PR**, and vice versa.
- **Review target: 48 hours.** A maintainer aims to leave a first review
  within two days of the PR being opened. Please don't force-push over a
  reviewed commit; push follow-up commits instead so the review history stays
  readable.

## Security issues

Do **not** open a public issue for a vulnerability. See [SECURITY.md](SECURITY.md)
for how to report privately.

## License

By contributing you agree that your contributions are licensed under the
[MIT License](LICENSE).
