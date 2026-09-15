# Security Policy

## Status

**The StellFlow escrow contract is unaudited and deployed to Stellar testnet
only. Do not use it with real funds.**

No third-party security review has been performed. The contract may contain
bugs that lose or lock funds. Any mainnet deployment before an audit is
unsupported.

## Reporting a vulnerability

Please report security issues **privately** by email to
**ojukwulevichinedu@gmail.com**. Do not open a public GitHub issue, discussion,
or pull request for a security problem.

Include as much of the following as you can:

- A description of the issue and its impact
- The affected function(s) in `escrow/contracts/escrow/src/`
- Steps or a test case that reproduces it (a failing test under
  `escrow/contracts/escrow/tests/` is ideal)
- Any suggested fix

You will receive an acknowledgement, and we will work with you on a fix and
disclosure timeline before anything is published.

## Scope

### In scope

- **Authorization bypass** — any path that lets an address other than the
  intended client, freelancer, or admin perform a restricted action
- **Fund lockup** — any state in which tokens held by the contract can no
  longer be released, refunded, or claimed by the rightful party
- **Arithmetic errors in fee or dispute-split calculation** — incorrect,
  over- or under-paying transfers in `release`, `release_milestone`,
  `resolve_dispute`, or the fee/treasury logic
- **State transitions that skip guards** — reaching a status (or milestone
  status) without passing the checks the state machine is meant to enforce

### Out of scope

- Fee optimisation or gas/resource-cost improvements
- Issues that require a compromised or malicious Stellar validator set
- Vulnerabilities in third-party dependencies with no demonstrated impact on
  this contract
- Issues in the testnet deployment itself (it is disposable and holds no
  value)

## Supported versions

Only the `main` branch is supported. There are no tagged releases yet.
