use soroban_sdk::contracterror;

/// Errors that can occur during escrow operations.
///
/// All errors are mapped to unique `u32` codes for on-chain error handling.
/// The contract uses these errors to enforce valid state transitions,
/// authorization checks, and input validation.
///
/// ## Error Categories
///
/// - **Input Validation**: `InvalidAmount`, `DeadlineInPast`, `MilestoneCountMismatch`, `MilestoneAmountMismatch`, `ZeroMilestones`
/// - **Authorization**: `Unauthorized`, `UnauthorizedAction`, `AdminRequired`
/// - **State Transitions**: `InvalidStateTransition`, `AlreadyFunded`, `EscrowAlreadyReleased`, `EscrowAlreadyRefunded`, `EscrowAlreadyCancelled`, `CannotModifyFundedEscrow`
/// - **Not Found**: `EscrowNotFound`, `MilestoneNotFound`
/// - **Financial**: `InsufficientBalance`, `TransferFailed`, `FeeTransferFailed`, `TreasuryNotConfigured`, `CannotSetFeeExceedingMax`
/// - **Deadline**: `DeadlineExpired`, `DeadlineNotPassed`
/// - **Milestone**: `MilestoneAlreadyReleased`, `CannotReleaseUnapprovedMilestone`, `CannotSubmitAlreadySubmittedMilestone`
/// - **Contract State**: `ContractPaused`, `InvalidToken`
/// - **Dispute**: `DisputeAlreadyRaised`, `NoActiveDispute`
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum EscrowError {
    /// Amount must be greater than zero.
    InvalidAmount = 1,
    /// Caller is not authorized for this operation.
    Unauthorized = 2,
    /// No escrow found with the given ID.
    EscrowNotFound = 3,
    /// Escrow has already been funded.
    AlreadyFunded = 4,
    /// Escrow is not in a valid state for this operation.
    InvalidStatus = 5,
    /// Escrow has already been released.
    AlreadyReleased = 6,
    /// Escrow has already been refunded.
    AlreadyRefunded = 7,
    /// Token transfer failed.
    TransferFailed = 8,
    /// Escrow with this ID already exists.
    EscrowAlreadyExists = 9,
    /// Insufficient token balance for the operation.
    InsufficientBalance = 10,
    /// State transition is not allowed from the current state.
    InvalidStateTransition = 11,
    /// Deadline has expired.
    DeadlineExpired = 12,
    /// Deadline has not yet passed.
    DeadlineNotPassed = 13,
    /// Escrow has already been cancelled.
    EscrowAlreadyCancelled = 14,
    /// Cannot modify an escrow that has been funded.
    CannotModifyFundedEscrow = 15,
    /// No milestone found with the given ID.
    MilestoneNotFound = 16,
    /// A dispute has already been raised for this escrow.
    DisputeAlreadyRaised = 17,
    /// No active dispute exists for this escrow.
    NoActiveDispute = 18,
    /// Fee transfer to treasury failed.
    FeeTransferFailed = 19,
    /// Fee percentage exceeds maximum allowed (10%).
    InvalidFeePercentage = 20,
    /// Admin has not been initialized.
    AdminRequired = 21,
    /// Contract is paused and cannot process operations.
    ContractPaused = 22,
    /// Token address is not a valid contract.
    InvalidToken = 23,
    /// Deadline cannot be in the past.
    DeadlineInPast = 24,
    /// Caller is not authorized for this specific action.
    UnauthorizedAction = 25,
    /// Total milestone amounts must equal escrow amount.
    MilestoneAmountMismatch = 26,
    /// Number of descriptions must equal number of amounts.
    MilestoneCountMismatch = 27,
    /// Milestone funds have already been released.
    MilestoneAlreadyReleased = 28,
    /// Cannot release a milestone that hasn't been approved.
    CannotReleaseUnapprovedMilestone = 29,
    /// Cannot submit a milestone that has already been submitted.
    CannotSubmitAlreadySubmittedMilestone = 30,
    /// Must provide at least one milestone.
    ZeroMilestones = 31,
    /// Escrow has already been released (terminal state).
    EscrowAlreadyReleased = 32,
    /// Escrow has already been refunded (terminal state).
    EscrowAlreadyRefunded = 33,
    /// Invalid deadline transition.
    InvalidDeadlineTransition = 34,
    /// Treasury address has not been configured.
    TreasuryNotConfigured = 35,
    /// Fee percentage cannot exceed the maximum allowed.
    CannotSetFeeExceedingMax = 36,
    /// Contract has already been initialized.
    AlreadyInitialized = 37,
    /// Version mismatch during migration.
    VersionMismatch = 38,
    /// Role has not been assigned to this address.
    RoleNotFound = 39,
    /// Address already has this role.
    RoleAlreadyAssigned = 40,
    /// Token does not implement required interface.
    InvalidTokenContract = 41,
}
