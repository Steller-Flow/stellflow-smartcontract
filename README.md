# StellFlow Escrow Contract

[![CI](https://github.com/Steller-Flow/stellflow-smartcontract/actions/workflows/ci.yml/badge.svg)](https://github.com/Steller-Flow/stellflow-smartcontract/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A Soroban smart contract for milestone-based escrow between a client and a
freelancer on Stellar. Funds in any Soroban token contract (for example a
Stellar Asset Contract wrapping USDC) are locked in the contract and released,
refunded, split by an admin on dispute, or reclaimed by the client after a
deadline.

> **⚠️ Unaudited — testnet only.** This contract has not had a third-party
> security review and is deployed to Stellar **testnet** only. Do not use it
> with real funds. See [SECURITY.md](SECURITY.md).

## Testnet deployment

| | |
|---|---|
| Contract ID | `CCXOOFWSH3REC6763NQLNGCGPJZE7JSVLLLCZWNLEDUPOP3LCOIWFPUI` |
| Explorer | [stellar.expert](https://stellar.expert/explorer/testnet/contract/CCXOOFWSH3REC6763NQLNGCGPJZE7JSVLLLCZWNLEDUPOP3LCOIWFPUI) · [lab.stellar.org](https://lab.stellar.org/r/testnet/contract/CCXOOFWSH3REC6763NQLNGCGPJZE7JSVLLLCZWNLEDUPOP3LCOIWFPUI) |
| WASM hash | `9c55e438dc8c46bd4232ee837a5660e8ef6bd03d222b72e4e95a4dc69574cc54` |
| Admin | `GA4V7OOAN2EIPSBDTKMKSD3BQ36FTZQ3XH6GSLIIAMX6TRM3NEDM5MIU` |
| Deploy tx | [`f30e883c…`](https://stellar.expert/explorer/testnet/tx/f30e883caff8d173f8ffb8db7827409520993d09e6df9c468bd16314bcf8d7dc) |
| Init tx | [`abfe2d59…`](https://stellar.expert/explorer/testnet/tx/abfe2d59648638f6a5ecc56da8fd9eb3c1e4ab2aa519684426d3a8feb654f07e) |

Read it yourself:

```bash
stellar contract invoke --id CCXOOFWSH3REC6763NQLNGCGPJZE7JSVLLLCZWNLEDUPOP3LCOIWFPUI \
  --network testnet --source <any-funded-testnet-key> -- get_admin
# "GA4V7OOAN2EIPSBDTKMKSD3BQ36FTZQ3XH6GSLIIAMX6TRM3NEDM5MIU"
```

## Capabilities

| Capability | Status | Notes |
|---|---|---|
| Single-payment escrow (create → fund → release / refund) | ✅ | |
| Milestone escrow (per-milestone submit / approve / reject / release) | ✅ | Milestone amounts must sum to the escrow amount |
| Cancel or modify before funding | ✅ | Pending state only |
| Deadlines with client timeout refund | ✅ | `set_deadline`, `claim_timeout` |
| Disputes with admin resolution (release / refund / split) | ✅ | Resolved by the contract admin |
| Platform fee (0–10 %) paid to a treasury | ✅ | Per-escrow and default fee |
| Any Soroban token per escrow | ✅ | Token address stored on each escrow |
| On-chain state-transition history per escrow | ✅ | `get_history` |
| Contract events for every transition | ✅ | 18 event topics |
| Emergency pause | ✅ | Blocks all non-admin mutations |
| Configurable storage TTL and cleanup of expired escrows | ✅ | |
| Version number and `migrate` | ✅ | Bumps the stored version; no WASM-upgrade hook |
| Role-based access control | ⚠️ partial | Roles can be assigned and queried, but no guard checks them — only the single admin address is enforced |
| Per-escrow arbiter | ⚠️ partial | `set_arbiter` stores an address, but `resolve_dispute` only accepts the admin |
| Security audit | ❌ | |
| Mainnet deployment | ❌ | |

See [Known limitations](#known-limitations) for the details behind the ⚠️
rows.

## State machine

```text
create_escrow / create_escrow_with_milestones
        │
        ▼
     Pending ──cancel_escrow──► Cancelled
        │  (modify_escrow, set_deadline allowed here)
        │ fund_escrow
        ▼
     Funded ───refund / claim_timeout──► Refunded
        │  (milestone submit/approve/reject/release,
        │   set_deadline allowed here)
        ├───release (after deadline, if set)──► Released
        │
        │ raise_dispute (client or freelancer)
        ▼
     Disputed ──resolve_dispute (admin)──► Released  (release or split)
                                        └► Refunded  (refund)
```

`Released`, `Refunded`, and `Cancelled` are terminal.

## Contract API

All 32 exported functions, grouped by lifecycle. Every mutating function that
takes an address as its first parameter calls `require_auth()` on it; the
"Authorized" column says who that address must be. Errors are
`EscrowError` variants from [`errors.rs`](escrow/contracts/escrow/src/errors.rs);
the numeric code (shown as `#N` in transaction diagnostics) is in
parentheses.

Unless noted, every mutating function also fails with `ContractPaused` (22)
while the contract is paused, and every function taking an `escrow_id` fails
with `EscrowNotFound` (3) for an unknown ID.

### Creation

#### `create_escrow(client: Address, freelancer: Address, token: Address, amount: i128, deadline: Option<u64>) -> Result<u64, EscrowError>`

Creates a `Pending` escrow and returns its ID (IDs start at 1 and increment).
The escrow's fee is snapshotted from the current default fee. The token is
checked by calling `balance()` on it — an address that is not a token
contract makes the call fail with a host error rather than an `EscrowError`.

- **Authorized:** `client`
- **Errors:** `InvalidAmount` (1) if `amount <= 0`; `UnauthorizedAction` (25)
  if `client == freelancer`; `DeadlineInPast` (24) if `deadline` is not after
  the current ledger timestamp

#### `create_escrow_with_milestones(client: Address, freelancer: Address, token: Address, amount: i128, milestone_descriptions: Vec<String>, milestone_amounts: Vec<i128>, deadline: Option<u64>) -> Result<u64, EscrowError>`

As `create_escrow`, but attaches milestones. Milestone `i` gets
`milestone_id = i`, status `Pending`, `released = false`.

- **Authorized:** `client`
- **Errors:** as `create_escrow`, plus `MilestoneCountMismatch` (27) if the
  two vectors differ in length; `ZeroMilestones` (31) if they are empty;
  `MilestoneAmountMismatch` (26) if the amounts do not sum to `amount`

### Funding

#### `fund_escrow(client: Address, escrow_id: u64) -> Result<(), EscrowError>`

Transfers `amount` of the escrow's token from `client` to the contract and
moves the escrow to `Funded`. Records `funded_at` and a history entry.

- **Authorized:** the escrow's `client`
- **Errors:** `UnauthorizedAction` (25) if not the client; `AlreadyFunded`
  (4); `EscrowAlreadyCancelled` (14); `EscrowAlreadyReleased` (32);
  `EscrowAlreadyRefunded` (33); `InvalidStateTransition` (11) if `Disputed`

### Settlement

#### `release(client: Address, escrow_id: u64) -> Result<(), EscrowError>`

Pays the freelancer `remaining - fee` — where `remaining` is `amount` minus
any milestone releases already paid out — and moves a `Funded` escrow to
`Released`.
If a fee applies and a treasury is configured, the fee is sent to the
treasury. **If a deadline is set, `release` is only allowed once the deadline
has passed.**

- **Authorized:** the escrow's `client`
- **Errors:** `UnauthorizedAction` (25); `InvalidStateTransition` (11) if
  `Pending`; `EscrowAlreadyReleased` (32); `EscrowAlreadyRefunded` (33);
  `EscrowAlreadyCancelled` (14); `NoActiveDispute` (18) if `Disputed`;
  `DeadlineNotPassed` (13)

#### `refund(client: Address, escrow_id: u64) -> Result<(), EscrowError>`

Returns the remaining balance (`amount` minus milestone releases already
paid out) to the client and moves a `Funded` escrow to `Refunded`. No fee is
taken.

- **Authorized:** the escrow's `client`
- **Errors:** as `release`, without `DeadlineNotPassed`

### Cancellation and modification

#### `cancel_escrow(client: Address, escrow_id: u64) -> Result<(), EscrowError>`

Moves a `Pending` (unfunded) escrow to `Cancelled`.

- **Authorized:** the escrow's `client`
- **Errors:** `UnauthorizedAction` (25); `InvalidStateTransition` (11) if
  `Funded`; `EscrowAlreadyReleased` (32); `EscrowAlreadyRefunded` (33);
  `EscrowAlreadyCancelled` (14); `NoActiveDispute` (18) if `Disputed`

#### `modify_escrow(client: Address, escrow_id: u64, new_freelancer: Option<Address>, new_amount: Option<i128>) -> Result<(), EscrowError>`

Updates the freelancer and/or amount of a `Pending` escrow. Fields passed as
`None` are left unchanged. Milestones are **not** adjusted.

- **Authorized:** the escrow's `client`
- **Errors:** `UnauthorizedAction` (25) if not the client or if
  `new_freelancer == client`; `CannotModifyFundedEscrow` (15);
  `EscrowAlreadyReleased` (32); `EscrowAlreadyRefunded` (33);
  `EscrowAlreadyCancelled` (14); `InvalidStateTransition` (11) if `Disputed`;
  `InvalidAmount` (1) if `new_amount <= 0`

### Milestones

All four require the escrow to be `Funded`. Milestone status lifecycle:
`Pending → Submitted → Approved` (then released) or `→ Rejected`.

#### `submit_milestone(freelancer: Address, escrow_id: u64, milestone_id: u32) -> Result<(), EscrowError>`

Marks a `Pending` milestone `Submitted`.

- **Authorized:** the escrow's `freelancer`
- **Errors:** `UnauthorizedAction` (25); `InvalidStateTransition` (11) if the
  escrow is not `Funded`; `CannotSubmitAlreadySubmittedMilestone` (30) if the
  milestone is not `Pending`; `MilestoneNotFound` (16)

#### `approve_milestone(client: Address, escrow_id: u64, milestone_id: u32) -> Result<(), EscrowError>`

Marks a `Pending` or `Submitted` milestone `Approved`.

- **Authorized:** the escrow's `client`
- **Errors:** `UnauthorizedAction` (25); `InvalidStateTransition` (11) if the
  escrow is not `Funded` or the milestone is already `Approved`;
  `CannotReleaseUnapprovedMilestone` (29) if the milestone is `Rejected`;
  `MilestoneNotFound` (16)

#### `reject_milestone(client: Address, escrow_id: u64, milestone_id: u32) -> Result<(), EscrowError>`

Marks a `Pending` or `Submitted` milestone `Rejected`.

- **Authorized:** the escrow's `client`
- **Errors:** `UnauthorizedAction` (25); `InvalidStateTransition` (11) if the
  escrow is not `Funded` or the milestone is already `Rejected`;
  `CannotReleaseUnapprovedMilestone` (29) if the milestone is `Approved`;
  `MilestoneNotFound` (16)

#### `release_milestone(client: Address, escrow_id: u64, milestone_id: u32) -> Result<(), EscrowError>`

Transfers an `Approved` milestone's amount to the freelancer, marks it
`released`, and adds it to `total_released`. No fee is taken on milestone
releases. The escrow stays `Funded`; a later `release`, `refund`,
`claim_timeout`, or `resolve_dispute` operates on what remains.

- **Authorized:** the escrow's `client`
- **Errors:** `UnauthorizedAction` (25); `InvalidStateTransition` (11) if the
  escrow is not `Funded`; `CannotReleaseUnapprovedMilestone` (29);
  `MilestoneAlreadyReleased` (28); `MilestoneNotFound` (16)

### Disputes

#### `raise_dispute(caller: Address, escrow_id: u64) -> Result<(), EscrowError>`

Moves a `Funded` escrow to `Disputed` and records `disputed_at`.

- **Authorized:** the escrow's `client` **or** `freelancer`
- **Errors:** `UnauthorizedAction` (25); `InvalidStateTransition` (11) if not
  `Funded`; `DisputeAlreadyRaised` (17)

#### `resolve_dispute(resolver: Address, escrow_id: u64, release_to_freelancer: bool, split_to_freelancer: Option<i128>) -> Result<(), EscrowError>`

Settles a `Disputed` escrow.

All three outcomes pay out exactly the remaining balance (`amount` minus
milestone releases already paid out). Decision order:

1. If `split_to_freelancer` is `Some(f)`: the fee is charged on the
   freelancer's share only — the freelancer receives `f - fee(f)`, the
   treasury receives `fee(f)`, the client receives `remaining - f`, and the
   escrow becomes `Released`. `release_to_freelancer` is ignored.
   `Some(remaining)` pays the same as outcome 2 and `Some(0)` the same as
   outcome 3.
2. Else if `release_to_freelancer` is `true`: as `release` — freelancer
   receives `remaining - fee`, treasury receives the fee, escrow becomes
   `Released`.
3. Else: the client receives the full remaining balance, escrow becomes
   `Refunded`.

- **Authorized:** the contract admin (the per-escrow arbiter is not consulted)
- **Errors:** `Unauthorized` (2) if no admin is initialized;
  `UnauthorizedAction` (25) if `resolver` is not the admin;
  `NoActiveDispute` (18) if not `Disputed`; `InvalidAmount` (1) if the split is
  negative or greater than the remaining balance

#### `set_arbiter(admin: Address, escrow_id: u64, arbiter: Address) -> Result<(), EscrowError>`

Stores an arbiter address on the escrow. Not checked by the pause flag.
Nothing currently reads this field.

- **Authorized:** the contract admin
- **Errors:** `AdminRequired` (21); `UnauthorizedAction` (25)

### Deadlines

#### `set_deadline(client: Address, escrow_id: u64, deadline: u64) -> Result<(), EscrowError>`

Sets or replaces the deadline (Unix seconds) on a `Pending` or `Funded`
escrow. Once set, the client cannot `release` before it and can
`claim_timeout` after it.

- **Authorized:** the escrow's `client`
- **Errors:** `UnauthorizedAction` (25); `EscrowAlreadyReleased` (32);
  `EscrowAlreadyRefunded` (33); `EscrowAlreadyCancelled` (14);
  `InvalidStateTransition` (11) if `Disputed`; `DeadlineInPast` (24)

#### `claim_timeout(client: Address, escrow_id: u64) -> Result<(), EscrowError>`

After the deadline has passed, returns the remaining balance (`amount` minus
milestone releases already paid out) to the client and moves a `Funded`
escrow to `Refunded`.

- **Authorized:** the escrow's `client`
- **Errors:** `UnauthorizedAction` (25); `InvalidStateTransition` (11) if not
  `Funded`; `DeadlineNotPassed` (13) if there is no deadline or it has not yet
  passed

### Fees and treasury

The fee is `fee_percent` of the amount being paid to the freelancer
(`paid * fee_percent / 100`, integer division),
`fee_percent` ∈ 0–10. New escrows copy the default fee at creation; the
default is 0. Fees are only ever moved to the treasury when one is
configured. Admin functions in this group are not blocked by the pause flag.

#### `set_fee(admin: Address, escrow_id: u64, fee_percent: u32) -> Result<(), EscrowError>`

Sets the fee percentage of one escrow (any status) and records a history
entry.

- **Authorized:** the contract admin
- **Errors:** `AdminRequired` (21); `UnauthorizedAction` (25);
  `CannotSetFeeExceedingMax` (36) if `fee_percent > 10`

#### `set_default_fee(admin: Address, fee_percent: u32) -> Result<(), EscrowError>`

Sets the fee percentage applied to escrows created from now on.

- **Authorized:** the contract admin
- **Errors:** `AdminRequired` (21); `UnauthorizedAction` (25);
  `CannotSetFeeExceedingMax` (36)

#### `set_treasury(admin: Address, treasury: Address) -> Result<(), EscrowError>`

Sets the address that receives fees.

- **Authorized:** the contract admin
- **Errors:** `AdminRequired` (21); `UnauthorizedAction` (25)

### Admin

Admin functions authenticate the `admin` argument and compare it with the
stored admin. They are **not** blocked by the pause flag.

#### `initialize_admin(admin: Address) -> Result<(), EscrowError>`

Sets the admin and the contract version (to 1). Can only succeed once.
**This function does not require any authorization** — whoever calls it first
becomes admin — so it must be called in the same deployment sequence as
`stellar contract deploy`.

- **Authorized:** anyone (first caller)
- **Errors:** `AlreadyInitialized` (37)

#### `set_paused(admin: Address, paused: bool) -> Result<(), EscrowError>`

Pauses or unpauses the contract. While paused, every function in the
Creation, Funding, Settlement, Cancellation, Milestones, Deadlines, and
Disputes groups (including `resolve_dispute`) fails with `ContractPaused`.

- **Authorized:** the contract admin
- **Errors:** `AdminRequired` (21); `UnauthorizedAction` (25)

#### `assign_role(admin: Address, address: Address, role: String) -> Result<(), EscrowError>`

Appends `role` to the list stored for `address` and emits a `RLE_ASG` event.
Roles are free-form strings; no contract guard checks them.

- **Authorized:** the contract admin
- **Errors:** `AdminRequired` (21); `UnauthorizedAction` (25);
  `RoleAlreadyAssigned` (40)

#### `set_escrow_ttl(admin: Address, ttl: u32) -> Result<(), EscrowError>`

Sets the TTL (in ledgers) applied to escrow storage entries and the contract
instance on each write. Default 2,000,000; allowed range
1,000,000–7,776,000.

- **Authorized:** the contract admin
- **Errors:** `AdminRequired` (21); `UnauthorizedAction` (25);
  `InvalidAmount` (1) if out of range

#### `cleanup_expired_escrows(admin: Address) -> Result<u32, EscrowError>`

Walks every escrow ID and removes those in a terminal state whose terminal
timestamp is older than `ttl × 5` seconds. Returns the number removed.

- **Authorized:** the contract admin
- **Errors:** `AdminRequired` (21); `UnauthorizedAction` (25)

#### `migrate(admin: Address, new_version: u32) -> Result<(), EscrowError>`

Reads every stored escrow (a storage integrity walk), sets the version to
`new_version`, and emits `CTR_UPG`. It does not change the contract's WASM.

- **Authorized:** the contract admin
- **Errors:** `AdminRequired` (21); `UnauthorizedAction` (25);
  `VersionMismatch` (38) if `new_version <= current`

### Read-only getters

No authorization required.

| Function | Returns |
|---|---|
| `get_escrow(escrow_id: u64) -> Result<Escrow, EscrowError>` | The full `Escrow` record; `EscrowNotFound` (3) |
| `get_history(escrow_id: u64) -> Result<Vec<EscrowEvent>, EscrowError>` | The escrow's `(from_status, to_status, actor, timestamp, amount)` transition log; `EscrowNotFound` (3) |
| `get_admin() -> Option<Address>` | The admin, or `None` before initialization |
| `is_paused() -> bool` | Pause flag |
| `get_escrow_ttl() -> u32` | Configured storage TTL in ledgers |
| `get_version() -> u32` | Stored version (0 before `initialize_admin`) |
| `has_role(address: Address, role: String) -> bool` | Whether `role` was assigned to `address` |

### Events

Every transition emits a contract event whose first topic is one of:
`ESC_CRT`, `ESC_FND`, `ESC_REL`, `ESC_RFD`, `ESC_CAN`, `ESC_DPT`, `ESC_RSV`,
`ESC_MDF`, `ESC_DLN`, `ESC_TMO`, `MSN_SUB`, `MSN_APR`, `MSN_RJT`, `MSN_REL`,
`FEE_COL`, `FEE_UPD`, `CTR_UPG`, `RLE_ASG`. See
[`events.rs`](escrow/contracts/escrow/src/events.rs) for payloads.

## Known limitations

These are behaviours of the current code, listed so reviewers don't have to
discover them. None are fixed yet.

- **`initialize_admin` is unauthenticated.** The first caller after
  deployment becomes admin. Deploy and initialize in one sequence.
- **Roles are not enforced.** `assign_role` / `has_role` store and report
  roles, but every admin-gated function checks only the single admin address.
- **The per-escrow arbiter is not used.** `resolve_dispute` accepts only the
  admin regardless of `set_arbiter`.
- **Fees are retained if no treasury is set.** `release` still deducts the
  fee from the freelancer's payout, but with no treasury the fee stays in the
  contract with no function to withdraw it.
- **`ContractPaused` does not apply to admin functions**, by design, so the
  admin can still change fees, roles, and the pause flag while paused.

## Tests

114 tests across 9 integration suites in
[`escrow/contracts/escrow/tests/`](escrow/contracts/escrow/tests/), run
against the Soroban test environment with a registered Stellar Asset Contract
as the token.

| Suite | Tests | Covers |
|---|---|---|
| `dispute_resolution_tests` | 16 | raise / resolve, split bounds, authorization |
| `release_refund_tests` | 14 | release, refund, wrong-client and terminal-state guards, timestamps, totals |
| `fee_and_history_tests` | 19 | fee on release, on dispute resolution and on splits, zero fee, treasury balances, history log, milestone balances, settlement after partial milestone release |
| `modify_escrow_tests` | 13 | modify before funding, validation |
| `timeout_tests` | 13 | deadline validation, `set_deadline`, `claim_timeout` success and guards |
| `create_escrow_tests` | 12 | creation, amount/deadline validation, sequential IDs, initial defaults |
| `cancel_escrow_tests` | 11 | cancel from each state |
| `integration_tests` | 10 | end-to-end lifecycles, milestone lifecycle and validation, pause blocks operations |
| `fund_escrow_tests` | 6 | funding, wrong-client and state guards |
| **Total** | **114** | |

```bash
cd escrow
cargo test --all
cargo test --all --features testutils
```

## Build

Soroban contracts must be built with `stellar contract build`, not
`cargo build` (which produces a native library, not a deployable WASM).

```bash
# one-time
rustup target add wasm32v1-none
cargo install --locked stellar-cli    # or: brew install stellar-cli

cd escrow
stellar contract build
# → target/wasm32v1-none/release/stellflow_escrow.wasm (~44 KB)
```

CI ([`.github/workflows/ci.yml`](.github/workflows/ci.yml)) runs `cargo fmt
--check`, `cargo clippy -D warnings`, both test commands, `cargo audit`, and
the WASM build, and uploads the `.wasm` as the `escrow-contract-wasm`
artifact.

### Deploy to testnet

```bash
stellar keys generate deployer --network testnet --fund
cd escrow
stellar contract build
stellar contract deploy \
  --wasm target/wasm32v1-none/release/stellflow_escrow.wasm \
  --source deployer --network testnet --alias my-escrow
# immediately, in the same session:
stellar contract invoke --id my-escrow --source deployer --network testnet \
  -- initialize_admin --admin "$(stellar keys address deployer)"
```

## Repository layout

```text
.
├── README.md, LICENSE, SECURITY.md, CONTRIBUTING.md
├── .github/workflows/ci.yml
└── escrow/                       # Cargo workspace (see escrow/README.md)
    └── contracts/escrow/
        ├── src/
        │   ├── lib.rs            # #![no_std] crate root
        │   ├── contract.rs       # the 32 exported functions
        │   ├── storage.rs        # storage keys, TTL, roles
        │   ├── events.rs         # event topics and emitters
        │   ├── types.rs          # Escrow, Milestone, EscrowEvent, enums
        │   ├── errors.rs         # EscrowError (41 variants)
        │   └── testutils.rs      # cfg(test) helpers
        └── tests/                # 9 suites, 114 tests
```

## Contributing and security

- [CONTRIBUTING.md](CONTRIBUTING.md) — setup, workflow, PR expectations
- [SECURITY.md](SECURITY.md) — report vulnerabilities privately

## License

[MIT](LICENSE) © 2026 StellFlow
