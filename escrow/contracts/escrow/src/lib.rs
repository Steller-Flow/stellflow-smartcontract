//! # StellFlow Escrow Contract
//!
//! A Soroban smart contract for milestone-based escrow on the Stellar blockchain.
//! Enables trustless payments between clients and freelancers using any supported
//! SPL token (not limited to USDC).
//!
//! ## Overview
//!
//! The escrow contract handles the full lifecycle of a payment agreement:
//!
//! 1. **Create** — A client creates an escrow specifying the freelancer, amount, and token.
//! 2. **Fund** — The client transfers tokens into the contract.
//! 3. **Release** — The client releases funds to the freelancer (with optional fee).
//! 4. **Refund** — The client reclaims funds if work isn't delivered.
//!
//! Additional features include dispute resolution, milestone tracking, timeout-based
//! auto-refund, and admin controls.
//!
//! ## State Machine
//!
//! ```text
//! Pending → Funded → Released
//!                 ↘ Refunded
//!                 ↘ Disputed → Released (via arbiter)
//!                            → Refunded (via arbiter)
//! Pending → Cancelled
//! ```
//!
//! ## Security Model
//!
//! - All mutating operations require authorization from the relevant party
//! - Only the client can fund, release, refund, cancel, or modify an escrow
//! - Disputes can be raised by either party
//! - Only the admin/arbiter can resolve disputes
//! - Contract can be paused by admin for emergency stops
//! - Role-based access control for admin operations
//!
//! ## Multi-Currency Support
//!
//! Each escrow specifies a token address at creation time. All token operations
//! (transfers, balance checks) use the stored token address, supporting any
//! SPL-compatible token on Stellar.
//!
//! ## Upgrade Mechanism
//!
//! The contract tracks its version number. Admin can call `migrate` to perform
//! version-specific state migrations when upgrading the contract logic.
//!
//! ## Role-Based Access Control
//!
//! The contract supports the following roles:
//! - `admin`: Full administrative access (dispute resolution, fee management)
//! - `fee_manager`: Can manage fee configurations
//! - `pause_controller`: Can pause/unpause the contract

#![no_std]

extern crate alloc;

pub mod contract;
pub mod errors;
pub mod events;
pub mod storage;
pub mod testutils;
pub mod types;

pub use contract::EscrowContract;
pub use errors::EscrowError;
pub use types::{DataKey, Escrow, EscrowEvent, EscrowStatus, Milestone, MilestoneStatus};
