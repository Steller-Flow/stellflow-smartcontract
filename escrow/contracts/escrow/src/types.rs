use soroban_sdk::{contracttype, Address};

/// Represents the current state of an escrow agreement.
///
/// ## State Machine
///
/// ```text
/// Pending → Funded → Released
///                 ↘ Refunded
///                 ↘ Disputed → Released (via arbiter)
///                            → Refunded (via arbiter)
/// Pending → Cancelled
/// ```
///
/// ## Transitions
///
/// - `Pending` → `Funded`: Client funds the escrow
/// - `Pending` → `Cancelled`: Client cancels before funding
/// - `Funded` → `Released`: Client releases funds to freelancer
/// - `Funded` → `Refunded`: Client reclaims funds
/// - `Funded` → `Disputed`: Either party raises a dispute
/// - `Disputed` → `Released`: Arbiter resolves in favor of freelancer
/// - `Disputed` → `Refunded`: Arbiter resolves in favor of client
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EscrowStatus {
    /// Escrow created but not yet funded by the client.
    Pending,
    /// Client has deposited tokens into the escrow.
    Funded,
    /// Funds released to the freelancer (terminal state).
    Released,
    /// Funds refunded to the client (terminal state).
    Refunded,
    /// Escrow cancelled by client before funding (terminal state).
    Cancelled,
    /// A dispute has been raised by either party.
    Disputed,
}

/// A single milestone within an escrow agreement.
///
/// Milestones allow escrow amounts to be split into deliverables,
/// each with independent approval and release cycles.
#[contracttype]
#[derive(Clone, Debug)]
pub struct Milestone {
    /// Unique identifier for this milestone within the escrow.
    pub milestone_id: u32,
    /// Human-readable description of the deliverable.
    pub description: soroban_sdk::String,
    /// Amount allocated to this milestone.
    pub amount: i128,
    /// Current approval status.
    pub status: MilestoneStatus,
    /// Whether funds for this milestone have been released.
    pub released: bool,
}

/// Status of an individual milestone.
///
/// ## Lifecycle
///
/// ```text
/// Pending → Submitted → Approved → Released
///                   ↘ Rejected
/// ```
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MilestoneStatus {
    /// Milestone created but not yet submitted by freelancer.
    Pending,
    /// Freelancer has submitted work for review.
    Submitted,
    /// Client has approved the milestone for release.
    Approved,
    /// Client has rejected the submission.
    Rejected,
}

/// The main escrow agreement stored on-chain.
///
/// Contains all state for a payment agreement between a client and freelancer,
/// including token information, amounts, milestones, and audit history.
#[contracttype]
#[derive(Clone, Debug)]
pub struct Escrow {
    /// Unique identifier for this escrow.
    pub escrow_id: u64,
    /// Address of the client (funder).
    pub client: Address,
    /// Address of the freelancer (recipient).
    pub freelancer: Address,
    /// Address of the SPL token contract used for this escrow.
    pub token: Address,
    /// Total escrow amount in base token units.
    pub amount: i128,
    /// Current status of the escrow.
    pub status: EscrowStatus,
    /// Timestamp when the escrow was created.
    pub created_at: u64,
    /// Timestamp when the escrow was funded (if funded).
    pub funded_at: Option<u64>,
    /// Timestamp when funds were released (if released).
    pub released_at: Option<u64>,
    /// Timestamp when funds were refunded (if refunded).
    pub refunded_at: Option<u64>,
    /// Timestamp when the escrow was cancelled (if cancelled).
    pub cancelled_at: Option<u64>,
    /// Timestamp when a dispute was raised (if disputed).
    pub disputed_at: Option<u64>,
    /// Optional deadline for the escrow (Unix timestamp).
    pub deadline: Option<u64>,
    /// List of milestones associated with this escrow.
    pub milestones: soroban_sdk::Vec<Milestone>,
    /// Optional arbiter address for dispute resolution.
    pub arbiter: Option<Address>,
    /// Platform fee percentage (0-10) applied to this escrow.
    pub fee_percent: u32,
    /// Total amount released to freelancer so far.
    pub total_released: i128,
    /// Total amount refunded to client so far.
    pub total_refunded: i128,
    /// Complete history of state transitions for this escrow.
    pub history: soroban_sdk::Vec<EscrowEvent>,
}

/// An event recording a single state transition in an escrow's lifecycle.
///
/// Stored in the escrow's history vector to provide a complete on-chain
/// audit trail without requiring off-chain indexing.
#[contracttype]
#[derive(Clone, Debug)]
pub struct EscrowEvent {
    /// Status before the transition.
    pub from_status: EscrowStatus,
    /// Status after the transition.
    pub to_status: EscrowStatus,
    /// Address of the actor who triggered the transition.
    pub actor: Address,
    /// Ledger timestamp of the transition.
    pub timestamp: u64,
    /// Amount involved in the transition (0 for non-financial operations).
    pub amount: i128,
}

/// Storage keys used by the escrow contract.
///
/// - `Escrow(u64)`: Persistent storage for individual escrow data.
/// - `EscrowCounter`: Instance storage for the auto-incrementing ID counter.
/// - `Admin`: Instance storage for the admin address.
/// - `Paused`: Instance storage for the pause flag.
/// - `PlatformTreasury`: Instance storage for the treasury address.
/// - `DefaultFeePercent`: Instance storage for the default fee percentage.
/// - `EscrowTTL`: Instance storage for the configurable TTL.
/// - `Version`: Instance storage for the contract version number.
#[contracttype]
pub enum DataKey {
    /// Persistent storage key for an escrow by ID.
    Escrow(u64),
    /// Auto-incrementing escrow ID counter.
    EscrowCounter,
    /// Contract admin address.
    Admin,
    /// Whether the contract is paused.
    Paused,
    /// Platform treasury address for fee collection.
    PlatformTreasury,
    /// Default fee percentage for new escrows.
    DefaultFeePercent,
    /// Configurable TTL for escrow storage.
    EscrowTTL,
    /// Contract version number for upgrade tracking.
    Version,
    /// Role-based access control key.
    Role(Address),
}
