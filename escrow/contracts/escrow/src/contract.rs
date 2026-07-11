use soroban_sdk::{contract, contractimpl, token, Address, Env, Vec};

use crate::{
    errors::EscrowError,
    events, storage,
    types::{Escrow, EscrowEvent, EscrowStatus, Milestone, MilestoneStatus},
};

const MAX_FEE_PERCENT: u32 = 10;

/// StellFlow Escrow Contract
///
/// A Soroban smart contract for milestone-based escrow on the Stellar blockchain.
/// Enables trustless payments between clients and freelancers using any supported token.
///
/// # State Machine
///
/// ```text
/// Pending → Funded → Released
///                 ↘ Refunded
///                 ↘ Disputed → Released (via arbiter)
///                            → Refunded (via arbiter)
/// Pending → Cancelled
/// ```
///
/// # Security Model
///
/// - All mutating operations require authorization from the relevant party
/// - Only the client can fund, release, refund, cancel, or modify an escrow
/// - Disputes can be raised by either party
/// - Only the admin/arbiter can resolve disputes
/// - Contract can be paused by admin for emergency stops
/// - Role-based access control for admin operations
///
/// # Upgrade Mechanism
///
/// The contract tracks its version number. Admin can call `migrate` to
/// perform version-specific state migrations when upgrading the contract.
#[contract]
pub struct EscrowContract;

#[contractimpl]
impl EscrowContract {
    #[allow(clippy::too_many_arguments)]
    pub fn create_escrow(
        env: Env,
        client: Address,
        freelancer: Address,
        token: Address,
        amount: i128,
        deadline: Option<u64>,
    ) -> Result<u64, EscrowError> {
        Self::require_not_paused(&env)?;
        Self::validate_token(&env, &token)?;
        if amount <= 0 {
            return Err(EscrowError::InvalidAmount);
        }
        if client == freelancer {
            return Err(EscrowError::UnauthorizedAction);
        }
        if let Some(dl) = deadline {
            if dl <= env.ledger().timestamp() {
                return Err(EscrowError::DeadlineInPast);
            }
        }
        client.require_auth();
        let escrow_id = storage::next_escrow_id(&env);
        let created_at = env.ledger().timestamp();
        let escrow = Escrow {
            escrow_id,
            client: client.clone(),
            freelancer: freelancer.clone(),
            token,
            amount,
            status: EscrowStatus::Pending,
            created_at,
            funded_at: None,
            released_at: None,
            refunded_at: None,
            cancelled_at: None,
            disputed_at: None,
            deadline,
            milestones: Vec::new(&env),
            arbiter: None,
            fee_percent: storage::get_default_fee_percent(&env),
            total_released: 0,
            total_refunded: 0,
            history: Vec::new(&env),
        };
        storage::save_escrow(&env, &escrow);
        events::emit_escrow_created(&env, escrow_id, &client, &freelancer, amount);
        Ok(escrow_id)
    }

    /// Creates a new escrow with milestone-based payment splits.
    ///
    /// The sum of milestone amounts must equal the total escrow amount.
    /// Each milestone can be independently submitted, approved, and released.
    ///
    /// # Arguments
    /// * `client` - Address of the client (must authorize)
    /// * `freelancer` - Address of the freelancer
    /// * `token` - Address of the SPL token contract
    /// * `amount` - Total escrow amount (must equal sum of milestone amounts)
    /// * `milestone_descriptions` - Descriptions for each milestone
    /// * `milestone_amounts` - Amounts for each milestone
    /// * `deadline` - Optional deadline timestamp
    ///
    /// # Errors
    /// - `MilestoneCountMismatch` if descriptions.len() != amounts.len()
    /// - `ZeroMilestones` if no milestones provided
    /// - `MilestoneAmountMismatch` if sum of amounts != total amount
    #[allow(clippy::too_many_arguments)]
    pub fn create_escrow_with_milestones(
        env: Env,
        client: Address,
        freelancer: Address,
        token: Address,
        amount: i128,
        milestone_descriptions: Vec<soroban_sdk::String>,
        milestone_amounts: Vec<i128>,
        deadline: Option<u64>,
    ) -> Result<u64, EscrowError> {
        Self::require_not_paused(&env)?;
        Self::validate_token(&env, &token)?;
        if amount <= 0 {
            return Err(EscrowError::InvalidAmount);
        }
        if client == freelancer {
            return Err(EscrowError::UnauthorizedAction);
        }
        if milestone_descriptions.len() != milestone_amounts.len() {
            return Err(EscrowError::MilestoneCountMismatch);
        }
        if milestone_descriptions.is_empty() {
            return Err(EscrowError::ZeroMilestones);
        }
        if let Some(dl) = deadline {
            if dl <= env.ledger().timestamp() {
                return Err(EscrowError::DeadlineInPast);
            }
        }
        let total_milestone_amount: i128 = milestone_amounts.iter().sum();
        if total_milestone_amount != amount {
            return Err(EscrowError::MilestoneAmountMismatch);
        }
        client.require_auth();
        let escrow_id = storage::next_escrow_id(&env);
        let created_at = env.ledger().timestamp();
        let mut milestones = Vec::new(&env);
        for i in 0..milestone_descriptions.len() {
            milestones.push_back(Milestone {
                milestone_id: i,
                description: milestone_descriptions.get(i).unwrap(),
                amount: milestone_amounts.get(i).unwrap(),
                status: MilestoneStatus::Pending,
                released: false,
            });
        }
        let escrow = Escrow {
            escrow_id,
            client: client.clone(),
            freelancer: freelancer.clone(),
            token,
            amount,
            status: EscrowStatus::Pending,
            created_at,
            funded_at: None,
            released_at: None,
            refunded_at: None,
            cancelled_at: None,
            disputed_at: None,
            deadline,
            milestones,
            arbiter: None,
            fee_percent: storage::get_default_fee_percent(&env),
            total_released: 0,
            total_refunded: 0,
            history: Vec::new(&env),
        };
        storage::save_escrow(&env, &escrow);
        events::emit_escrow_created(&env, escrow_id, &client, &freelancer, amount);
        Ok(escrow_id)
    }

    /// Funds an escrow by transferring tokens from the client to the contract.
    ///
    /// Transitions from `Pending` to `Funded` state.
    /// The client must have sufficient token balance.
    pub fn fund_escrow(env: Env, client: Address, escrow_id: u64) -> Result<(), EscrowError> {
        Self::require_not_paused(&env)?;
        client.require_auth();
        let mut escrow = storage::get_escrow(&env, escrow_id)?;
        if escrow.client != client {
            return Err(EscrowError::UnauthorizedAction);
        }
        match escrow.status {
            EscrowStatus::Pending => {}
            EscrowStatus::Funded => return Err(EscrowError::AlreadyFunded),
            EscrowStatus::Cancelled => return Err(EscrowError::EscrowAlreadyCancelled),
            EscrowStatus::Released => return Err(EscrowError::EscrowAlreadyReleased),
            EscrowStatus::Refunded => return Err(EscrowError::EscrowAlreadyRefunded),
            EscrowStatus::Disputed => return Err(EscrowError::InvalidStateTransition),
        }
        let token_client = token::Client::new(&env, &escrow.token);
        let escrow_amount = escrow.amount;
        token_client.transfer(&client, env.current_contract_address(), &escrow_amount);
        let old_status = escrow.status.clone();
        escrow.status = EscrowStatus::Funded;
        escrow.funded_at = Some(env.ledger().timestamp());
        Self::push_history(&env, &mut escrow, old_status, &client, escrow_amount);
        storage::save_escrow(&env, &escrow);
        events::emit_escrow_funded(&env, escrow_id, escrow_amount);
        Ok(())
    }

    /// Releases escrowed funds to the freelancer.
    ///
    /// Transitions from `Funded` to `Released` state.
    /// Deducts platform fee if configured. Only callable by the client.
    ///
    /// # Errors
    /// - `DeadlineNotPassed` if a deadline is set and hasn't passed
    /// - `UnauthorizedAction` if caller is not the client
    pub fn release(env: Env, client: Address, escrow_id: u64) -> Result<(), EscrowError> {
        Self::require_not_paused(&env)?;
        client.require_auth();
        let mut escrow = storage::get_escrow(&env, escrow_id)?;
        if escrow.client != client {
            return Err(EscrowError::UnauthorizedAction);
        }
        match escrow.status {
            EscrowStatus::Funded => {}
            EscrowStatus::Pending => return Err(EscrowError::InvalidStateTransition),
            EscrowStatus::Released => return Err(EscrowError::EscrowAlreadyReleased),
            EscrowStatus::Refunded => return Err(EscrowError::EscrowAlreadyRefunded),
            EscrowStatus::Cancelled => return Err(EscrowError::EscrowAlreadyCancelled),
            EscrowStatus::Disputed => return Err(EscrowError::NoActiveDispute),
        }
        if let Some(dl) = escrow.deadline {
            if env.ledger().timestamp() < dl {
                return Err(EscrowError::DeadlineNotPassed);
            }
        }
        let fee = Self::calculate_fee(&escrow);
        let release_amount = escrow.amount - fee;
        let token_client = token::Client::new(&env, &escrow.token);
        token_client.transfer(
            &env.current_contract_address(),
            &escrow.freelancer,
            &release_amount,
        );
        if fee > 0 {
            if let Some(treasury) = storage::get_treasury(&env) {
                token_client.transfer(&env.current_contract_address(), &treasury, &fee);
                events::emit_fee_collected(&env, escrow_id, fee, &treasury);
            }
        }
        let old_status = escrow.status.clone();
        escrow.status = EscrowStatus::Released;
        escrow.released_at = Some(env.ledger().timestamp());
        escrow.total_released = release_amount;
        Self::push_history(&env, &mut escrow, old_status, &client, release_amount);
        storage::save_escrow(&env, &escrow);
        events::emit_escrow_released(&env, escrow_id, &client, release_amount);
        Ok(())
    }

    /// Refunds escrowed funds back to the client.
    ///
    /// Transitions from `Funded` to `Refunded` state.
    /// Only callable by the client.
    pub fn refund(env: Env, client: Address, escrow_id: u64) -> Result<(), EscrowError> {
        Self::require_not_paused(&env)?;
        client.require_auth();
        let mut escrow = storage::get_escrow(&env, escrow_id)?;
        if escrow.client != client {
            return Err(EscrowError::UnauthorizedAction);
        }
        match escrow.status {
            EscrowStatus::Funded => {}
            EscrowStatus::Pending => return Err(EscrowError::InvalidStateTransition),
            EscrowStatus::Released => return Err(EscrowError::EscrowAlreadyReleased),
            EscrowStatus::Refunded => return Err(EscrowError::EscrowAlreadyRefunded),
            EscrowStatus::Cancelled => return Err(EscrowError::EscrowAlreadyCancelled),
            EscrowStatus::Disputed => return Err(EscrowError::NoActiveDispute),
        }
        let token_client = token::Client::new(&env, &escrow.token);
        let refund_amount = escrow.amount;
        token_client.transfer(&env.current_contract_address(), &client, &refund_amount);
        let old_status = escrow.status.clone();
        escrow.status = EscrowStatus::Refunded;
        escrow.refunded_at = Some(env.ledger().timestamp());
        escrow.total_refunded = refund_amount;
        Self::push_history(&env, &mut escrow, old_status, &client, refund_amount);
        storage::save_escrow(&env, &escrow);
        events::emit_escrow_refunded(&env, escrow_id, &client, refund_amount);
        Ok(())
    }

    /// Cancels a pending escrow before it is funded.
    ///
    /// Transitions from `Pending` to `Cancelled` state.
    /// Only callable by the client.
    pub fn cancel_escrow(env: Env, client: Address, escrow_id: u64) -> Result<(), EscrowError> {
        Self::require_not_paused(&env)?;
        client.require_auth();
        let mut escrow = storage::get_escrow(&env, escrow_id)?;
        if escrow.client != client {
            return Err(EscrowError::UnauthorizedAction);
        }
        match escrow.status {
            EscrowStatus::Pending => {}
            EscrowStatus::Funded => return Err(EscrowError::InvalidStateTransition),
            EscrowStatus::Released => return Err(EscrowError::EscrowAlreadyReleased),
            EscrowStatus::Refunded => return Err(EscrowError::EscrowAlreadyRefunded),
            EscrowStatus::Cancelled => return Err(EscrowError::EscrowAlreadyCancelled),
            EscrowStatus::Disputed => return Err(EscrowError::NoActiveDispute),
        }
        let old_status = escrow.status.clone();
        escrow.status = EscrowStatus::Cancelled;
        escrow.cancelled_at = Some(env.ledger().timestamp());
        Self::push_history(&env, &mut escrow, old_status, &client, 0);
        storage::save_escrow(&env, &escrow);
        events::emit_escrow_cancelled(&env, escrow_id, &client);
        Ok(())
    }

    /// Modifies an escrow's freelancer or amount before funding.
    ///
    /// Only allowed in `Pending` state. Both parameters are optional;
    /// only provided values will be updated.
    pub fn modify_escrow(
        env: Env,
        client: Address,
        escrow_id: u64,
        new_freelancer: Option<Address>,
        new_amount: Option<i128>,
    ) -> Result<(), EscrowError> {
        Self::require_not_paused(&env)?;
        client.require_auth();
        let mut escrow = storage::get_escrow(&env, escrow_id)?;
        if escrow.client != client {
            return Err(EscrowError::UnauthorizedAction);
        }
        match escrow.status {
            EscrowStatus::Pending => {}
            EscrowStatus::Funded => return Err(EscrowError::CannotModifyFundedEscrow),
            EscrowStatus::Released => return Err(EscrowError::EscrowAlreadyReleased),
            EscrowStatus::Refunded => return Err(EscrowError::EscrowAlreadyRefunded),
            EscrowStatus::Cancelled => return Err(EscrowError::EscrowAlreadyCancelled),
            EscrowStatus::Disputed => return Err(EscrowError::InvalidStateTransition),
        }
        if let Some(ref freelancer) = new_freelancer {
            if client == *freelancer {
                return Err(EscrowError::UnauthorizedAction);
            }
            escrow.freelancer = freelancer.clone();
        }
        if let Some(amount) = new_amount {
            if amount <= 0 {
                return Err(EscrowError::InvalidAmount);
            }
            escrow.amount = amount;
        }
        let old_status = escrow.status.clone();
        Self::push_history(&env, &mut escrow, old_status, &client, 0);
        storage::save_escrow(&env, &escrow);
        events::emit_escrow_modified(
            &env,
            escrow_id,
            &client,
            new_amount,
            new_freelancer.as_ref(),
        );
        Ok(())
    }

    /// Sets a deadline for the escrow.
    ///
    /// Allowed in `Pending` or `Funded` states.
    /// Deadline must be in the future.
    pub fn set_deadline(
        env: Env,
        client: Address,
        escrow_id: u64,
        deadline: u64,
    ) -> Result<(), EscrowError> {
        Self::require_not_paused(&env)?;
        client.require_auth();
        let mut escrow = storage::get_escrow(&env, escrow_id)?;
        if escrow.client != client {
            return Err(EscrowError::UnauthorizedAction);
        }
        match escrow.status {
            EscrowStatus::Pending | EscrowStatus::Funded => {}
            EscrowStatus::Released => return Err(EscrowError::EscrowAlreadyReleased),
            EscrowStatus::Refunded => return Err(EscrowError::EscrowAlreadyRefunded),
            EscrowStatus::Cancelled => return Err(EscrowError::EscrowAlreadyCancelled),
            EscrowStatus::Disputed => return Err(EscrowError::InvalidStateTransition),
        }
        if deadline <= env.ledger().timestamp() {
            return Err(EscrowError::DeadlineInPast);
        }
        escrow.deadline = Some(deadline);
        let old_status = escrow.status.clone();
        Self::push_history(&env, &mut escrow, old_status, &client, 0);
        storage::save_escrow(&env, &escrow);
        events::emit_escrow_deadline_set(&env, escrow_id, &client, deadline);
        Ok(())
    }

    /// Claims a refund after the deadline has passed.
    ///
    /// Only callable by the client on a `Funded` escrow with a deadline.
    /// The deadline must have passed for the claim to succeed.
    pub fn claim_timeout(env: Env, client: Address, escrow_id: u64) -> Result<(), EscrowError> {
        Self::require_not_paused(&env)?;
        client.require_auth();
        let mut escrow = storage::get_escrow(&env, escrow_id)?;
        if escrow.client != client {
            return Err(EscrowError::UnauthorizedAction);
        }
        if escrow.status != EscrowStatus::Funded {
            return Err(EscrowError::InvalidStateTransition);
        }
        let deadline = escrow.deadline.ok_or(EscrowError::DeadlineNotPassed)?;
        if env.ledger().timestamp() < deadline {
            return Err(EscrowError::DeadlineNotPassed);
        }
        let token_client = token::Client::new(&env, &escrow.token);
        let claim_amount = escrow.amount;
        token_client.transfer(&env.current_contract_address(), &client, &claim_amount);
        let old_status = escrow.status.clone();
        escrow.status = EscrowStatus::Refunded;
        escrow.refunded_at = Some(env.ledger().timestamp());
        escrow.total_refunded = claim_amount;
        Self::push_history(&env, &mut escrow, old_status, &client, claim_amount);
        storage::save_escrow(&env, &escrow);
        events::emit_escrow_timeout_claimed(&env, escrow_id, &client, claim_amount);
        Ok(())
    }

    /// Approves a milestone for release.
    ///
    /// Transitions milestone from `Pending` or `Submitted` to `Approved`.
    /// Only callable by the client.
    pub fn approve_milestone(
        env: Env,
        client: Address,
        escrow_id: u64,
        milestone_id: u32,
    ) -> Result<(), EscrowError> {
        Self::require_not_paused(&env)?;
        client.require_auth();
        let mut escrow = storage::get_escrow(&env, escrow_id)?;
        if escrow.client != client {
            return Err(EscrowError::UnauthorizedAction);
        }
        if escrow.status != EscrowStatus::Funded {
            return Err(EscrowError::InvalidStateTransition);
        }
        let mut milestones = escrow.milestones.clone();
        let mut found = false;
        for i in 0..milestones.len() {
            let mut m = milestones.get(i).unwrap();
            if m.milestone_id == milestone_id {
                if m.status == MilestoneStatus::Approved {
                    return Err(EscrowError::InvalidStateTransition);
                }
                if m.status != MilestoneStatus::Pending && m.status != MilestoneStatus::Submitted {
                    return Err(EscrowError::CannotReleaseUnapprovedMilestone);
                }
                m.status = MilestoneStatus::Approved;
                milestones.set(i, m);
                found = true;
                events::emit_milestone_approved(&env, escrow_id, milestone_id, &client);
                break;
            }
        }
        if !found {
            return Err(EscrowError::MilestoneNotFound);
        }
        escrow.milestones = milestones;
        storage::save_escrow(&env, &escrow);
        Ok(())
    }

    /// Rejects a milestone submission.
    ///
    /// Transitions milestone from `Pending` or `Submitted` to `Rejected`.
    /// Only callable by the client.
    pub fn reject_milestone(
        env: Env,
        client: Address,
        escrow_id: u64,
        milestone_id: u32,
    ) -> Result<(), EscrowError> {
        Self::require_not_paused(&env)?;
        client.require_auth();
        let mut escrow = storage::get_escrow(&env, escrow_id)?;
        if escrow.client != client {
            return Err(EscrowError::UnauthorizedAction);
        }
        if escrow.status != EscrowStatus::Funded {
            return Err(EscrowError::InvalidStateTransition);
        }
        let mut milestones = escrow.milestones.clone();
        let mut found = false;
        for i in 0..milestones.len() {
            let mut m = milestones.get(i).unwrap();
            if m.milestone_id == milestone_id {
                if m.status == MilestoneStatus::Rejected {
                    return Err(EscrowError::InvalidStateTransition);
                }
                if m.status != MilestoneStatus::Pending && m.status != MilestoneStatus::Submitted {
                    return Err(EscrowError::CannotReleaseUnapprovedMilestone);
                }
                m.status = MilestoneStatus::Rejected;
                milestones.set(i, m);
                found = true;
                events::emit_milestone_rejected(&env, escrow_id, milestone_id, &client);
                break;
            }
        }
        if !found {
            return Err(EscrowError::MilestoneNotFound);
        }
        escrow.milestones = milestones;
        storage::save_escrow(&env, &escrow);
        Ok(())
    }

    /// Submits a milestone for client review.
    ///
    /// Transitions milestone from `Pending` to `Submitted`.
    /// Only callable by the freelancer.
    pub fn submit_milestone(
        env: Env,
        freelancer: Address,
        escrow_id: u64,
        milestone_id: u32,
    ) -> Result<(), EscrowError> {
        Self::require_not_paused(&env)?;
        freelancer.require_auth();
        let mut escrow = storage::get_escrow(&env, escrow_id)?;
        if escrow.freelancer != freelancer {
            return Err(EscrowError::UnauthorizedAction);
        }
        if escrow.status != EscrowStatus::Funded {
            return Err(EscrowError::InvalidStateTransition);
        }
        let mut milestones = escrow.milestones.clone();
        let mut found = false;
        for i in 0..milestones.len() {
            let mut m = milestones.get(i).unwrap();
            if m.milestone_id == milestone_id {
                if m.status != MilestoneStatus::Pending {
                    return Err(EscrowError::CannotSubmitAlreadySubmittedMilestone);
                }
                m.status = MilestoneStatus::Submitted;
                milestones.set(i, m);
                found = true;
                events::emit_milestone_submitted(&env, escrow_id, milestone_id, &freelancer);
                break;
            }
        }
        if !found {
            return Err(EscrowError::MilestoneNotFound);
        }
        escrow.milestones = milestones;
        storage::save_escrow(&env, &escrow);
        Ok(())
    }

    /// Releases funds for an approved milestone to the freelancer.
    ///
    /// Only callable by the client on an `Approved` milestone.
    /// Transfers the milestone amount to the freelancer.
    pub fn release_milestone(
        env: Env,
        client: Address,
        escrow_id: u64,
        milestone_id: u32,
    ) -> Result<(), EscrowError> {
        Self::require_not_paused(&env)?;
        client.require_auth();
        let mut escrow = storage::get_escrow(&env, escrow_id)?;
        if escrow.client != client {
            return Err(EscrowError::UnauthorizedAction);
        }
        if escrow.status != EscrowStatus::Funded {
            return Err(EscrowError::InvalidStateTransition);
        }
        let mut milestones = escrow.milestones.clone();
        let mut found = false;
        let mut milestone_amount = 0i128;
        for i in 0..milestones.len() {
            let mut m = milestones.get(i).unwrap();
            if m.milestone_id == milestone_id {
                if m.status != MilestoneStatus::Approved {
                    return Err(EscrowError::CannotReleaseUnapprovedMilestone);
                }
                if m.released {
                    return Err(EscrowError::MilestoneAlreadyReleased);
                }
                milestone_amount = m.amount;
                m.released = true;
                milestones.set(i, m);
                found = true;
                break;
            }
        }
        if !found {
            return Err(EscrowError::MilestoneNotFound);
        }
        escrow.milestones = milestones;
        escrow.total_released += milestone_amount;
        Self::push_history(
            &env,
            &mut escrow,
            EscrowStatus::Funded,
            &client,
            milestone_amount,
        );
        storage::save_escrow(&env, &escrow);
        let token_client = token::Client::new(&env, &escrow.token);
        token_client.transfer(
            &env.current_contract_address(),
            &escrow.freelancer,
            &milestone_amount,
        );
        events::emit_milestone_released(&env, escrow_id, milestone_id, &client, milestone_amount);
        Ok(())
    }

    /// Raises a dispute on a funded escrow.
    ///
    /// Either the client or freelancer can raise a dispute.
    /// Transitions from `Funded` to `Disputed` state.
    pub fn raise_dispute(env: Env, caller: Address, escrow_id: u64) -> Result<(), EscrowError> {
        Self::require_not_paused(&env)?;
        caller.require_auth();
        let mut escrow = storage::get_escrow(&env, escrow_id)?;
        if escrow.client != caller && escrow.freelancer != caller {
            return Err(EscrowError::UnauthorizedAction);
        }
        if escrow.status != EscrowStatus::Funded {
            return Err(EscrowError::InvalidStateTransition);
        }
        if escrow.disputed_at.is_some() {
            return Err(EscrowError::DisputeAlreadyRaised);
        }
        let old_status = escrow.status.clone();
        escrow.status = EscrowStatus::Disputed;
        escrow.disputed_at = Some(env.ledger().timestamp());
        Self::push_history(&env, &mut escrow, old_status, &caller, 0);
        storage::save_escrow(&env, &escrow);
        events::emit_escrow_disputed(&env, escrow_id, &caller, escrow.amount);
        Ok(())
    }

    /// Resolves a dispute by the admin/arbiter.
    ///
    /// Can release funds to freelancer, refund to client, or split the funds.
    /// Only callable by the admin address.
    #[allow(clippy::too_many_arguments)]
    pub fn resolve_dispute(
        env: Env,
        resolver: Address,
        escrow_id: u64,
        release_to_freelancer: bool,
        split_to_freelancer: Option<i128>,
    ) -> Result<(), EscrowError> {
        Self::require_not_paused(&env)?;
        resolver.require_auth();
        let arbiter = storage::get_admin(&env).ok_or(EscrowError::Unauthorized)?;
        if resolver != arbiter {
            return Err(EscrowError::UnauthorizedAction);
        }
        let mut escrow = storage::get_escrow(&env, escrow_id)?;
        if escrow.status != EscrowStatus::Disputed {
            return Err(EscrowError::NoActiveDispute);
        }
        let token_client = token::Client::new(&env, &escrow.token);
        let escrow_amount = escrow.amount;
        let escrow_client = escrow.client.clone();
        let escrow_freelancer = escrow.freelancer.clone();

        if let Some(freelancer_amount) = split_to_freelancer {
            if freelancer_amount < 0 || freelancer_amount > escrow_amount {
                return Err(EscrowError::InvalidAmount);
            }
            let client_amount = escrow_amount - freelancer_amount;
            if freelancer_amount > 0 {
                token_client.transfer(
                    &env.current_contract_address(),
                    &escrow_freelancer,
                    &freelancer_amount,
                );
            }
            if client_amount > 0 {
                token_client.transfer(
                    &env.current_contract_address(),
                    &escrow_client,
                    &client_amount,
                );
            }
            let fee = Self::calculate_fee(&escrow);
            let net_freelancer = freelancer_amount - fee;
            if fee > 0 && net_freelancer > 0 {
                if let Some(treasury) = storage::get_treasury(&env) {
                    token_client.transfer(&env.current_contract_address(), &treasury, &fee);
                    events::emit_fee_collected(&env, escrow_id, fee, &treasury);
                }
            }
            let old_status = escrow.status.clone();
            escrow.status = EscrowStatus::Released;
            escrow.released_at = Some(env.ledger().timestamp());
            escrow.total_released = net_freelancer;
            escrow.total_refunded = client_amount;
            Self::push_history(&env, &mut escrow, old_status, &resolver, escrow_amount);
            events::emit_escrow_resolved(
                &env,
                escrow_id,
                &resolver,
                "split_funds",
                net_freelancer,
                client_amount,
            );
        } else if release_to_freelancer {
            let fee = Self::calculate_fee(&escrow);
            let release_amount = escrow_amount - fee;
            token_client.transfer(
                &env.current_contract_address(),
                &escrow_freelancer,
                &release_amount,
            );
            if fee > 0 {
                if let Some(treasury) = storage::get_treasury(&env) {
                    token_client.transfer(&env.current_contract_address(), &treasury, &fee);
                    events::emit_fee_collected(&env, escrow_id, fee, &treasury);
                }
            }
            let old_status = escrow.status.clone();
            escrow.status = EscrowStatus::Released;
            escrow.released_at = Some(env.ledger().timestamp());
            escrow.total_released = release_amount;
            Self::push_history(&env, &mut escrow, old_status, &resolver, release_amount);
            events::emit_escrow_resolved(
                &env,
                escrow_id,
                &resolver,
                "released_to_freelancer",
                release_amount,
                0,
            );
        } else {
            token_client.transfer(
                &env.current_contract_address(),
                &escrow_client,
                &escrow_amount,
            );
            let old_status = escrow.status.clone();
            escrow.status = EscrowStatus::Refunded;
            escrow.refunded_at = Some(env.ledger().timestamp());
            escrow.total_refunded = escrow_amount;
            Self::push_history(&env, &mut escrow, old_status, &resolver, escrow_amount);
            events::emit_escrow_resolved(
                &env,
                escrow_id,
                &resolver,
                "refunded_to_client",
                0,
                escrow_amount,
            );
        }
        storage::save_escrow(&env, &escrow);
        Ok(())
    }

    /// Sets the arbiter for a specific escrow.
    ///
    /// Only callable by the admin.
    pub fn set_arbiter(
        env: Env,
        admin: Address,
        escrow_id: u64,
        arbiter: Address,
    ) -> Result<(), EscrowError> {
        Self::require_admin(&env, &admin)?;
        let mut escrow = storage::get_escrow(&env, escrow_id)?;
        escrow.arbiter = Some(arbiter);
        storage::save_escrow(&env, &escrow);
        Ok(())
    }

    /// Sets the fee percentage for a specific escrow.
    ///
    /// Only callable by the admin. Fee cannot exceed 10%.
    pub fn set_fee(
        env: Env,
        admin: Address,
        escrow_id: u64,
        fee_percent: u32,
    ) -> Result<(), EscrowError> {
        Self::require_admin(&env, &admin)?;
        if fee_percent > MAX_FEE_PERCENT {
            return Err(EscrowError::CannotSetFeeExceedingMax);
        }
        let mut escrow = storage::get_escrow(&env, escrow_id)?;
        let old_fee = escrow.fee_percent;
        escrow.fee_percent = fee_percent;
        let old_status = escrow.status.clone();
        Self::push_history(&env, &mut escrow, old_status, &admin, 0);
        storage::save_escrow(&env, &escrow);
        events::emit_fee_updated(&env, escrow_id, &admin, old_fee, fee_percent);
        Ok(())
    }

    /// Sets the default fee percentage for all new escrows.
    ///
    /// Only callable by the admin. Fee cannot exceed 10%.
    pub fn set_default_fee(env: Env, admin: Address, fee_percent: u32) -> Result<(), EscrowError> {
        Self::require_admin(&env, &admin)?;
        if fee_percent > MAX_FEE_PERCENT {
            return Err(EscrowError::CannotSetFeeExceedingMax);
        }
        storage::set_default_fee_percent(&env, fee_percent);
        Ok(())
    }

    /// Sets the treasury address for fee collection.
    ///
    /// Only callable by the admin.
    pub fn set_treasury(env: Env, admin: Address, treasury: Address) -> Result<(), EscrowError> {
        Self::require_admin(&env, &admin)?;
        storage::set_treasury(&env, &treasury);
        Ok(())
    }

    /// Initializes the contract admin.
    ///
    /// Can only be called once. The admin is responsible for dispute resolution,
    /// fee management, and contract administration.
    pub fn initialize_admin(env: Env, admin: Address) -> Result<(), EscrowError> {
        if storage::get_admin(&env).is_some() {
            return Err(EscrowError::AlreadyInitialized);
        }
        storage::set_admin(&env, &admin);
        storage::set_version(&env, storage::CURRENT_VERSION);
        Ok(())
    }

    /// Pauses or unpauses the contract.
    ///
    /// When paused, all mutating operations (except admin functions) are blocked.
    /// Only callable by the admin.
    pub fn set_paused(env: Env, admin: Address, paused: bool) -> Result<(), EscrowError> {
        Self::require_admin(&env, &admin)?;
        storage::set_paused(&env, paused);
        Ok(())
    }

    /// Returns the full escrow data for a given ID.
    ///
    /// # Errors
    /// Returns `EscrowError::EscrowNotFound` if no escrow exists.
    pub fn get_escrow(env: Env, escrow_id: u64) -> Result<Escrow, EscrowError> {
        storage::get_escrow(&env, escrow_id)
    }

    /// Returns the complete history of state transitions for an escrow.
    ///
    /// # Errors
    /// Returns `EscrowError::EscrowNotFound` if no escrow exists.
    pub fn get_history(env: Env, escrow_id: u64) -> Result<Vec<EscrowEvent>, EscrowError> {
        let escrow = storage::get_escrow(&env, escrow_id)?;
        Ok(escrow.history)
    }

    /// Returns the admin address, or `None` if not initialized.
    pub fn get_admin(env: Env) -> Option<Address> {
        storage::get_admin(&env)
    }

    /// Returns whether the contract is currently paused.
    pub fn is_paused(env: Env) -> bool {
        storage::is_paused(&env)
    }

    /// Sets the configurable TTL for escrow storage.
    ///
    /// Only callable by the admin. TTL must be between 1,000,000 and 7,776,000
    /// ledger increments.
    pub fn set_escrow_ttl(env: Env, admin: Address, ttl: u32) -> Result<(), EscrowError> {
        Self::require_admin(&env, &admin)?;
        storage::set_escrow_ttl(&env, ttl)
    }

    /// Returns the current configured TTL for escrow storage.
    pub fn get_escrow_ttl(env: Env) -> u32 {
        storage::get_escrow_ttl(&env)
    }

    /// Cleans up expired terminal escrows from storage.
    ///
    /// Only removes escrows in terminal states that have exceeded the TTL.
    /// Returns the number of escrows cleaned up.
    pub fn cleanup_expired_escrows(env: Env, admin: Address) -> Result<u32, EscrowError> {
        Self::require_admin(&env, &admin)?;
        storage::cleanup_expired_escrows(&env, &admin)
    }

    /// Returns the current contract version number.
    pub fn get_version(env: Env) -> u32 {
        storage::get_version(&env)
    }

    /// Performs a contract migration to a new version.
    ///
    /// Only callable by the admin. Version must be greater than current.
    /// Currently validates state integrity and updates the version number.
    ///
    /// # Errors
    /// - `VersionMismatch` if new_version <= current_version
    /// - `AdminRequired` if admin not initialized
    pub fn migrate(env: Env, admin: Address, new_version: u32) -> Result<(), EscrowError> {
        Self::require_admin(&env, &admin)?;
        let current = storage::get_version(&env);
        if new_version <= current {
            return Err(EscrowError::VersionMismatch);
        }
        let counter = storage::read_counter(&env);
        for id in 1..=counter {
            let _ = storage::get_escrow(&env, id);
        }
        storage::set_version(&env, new_version);
        events::emit_contract_upgraded(&env, &admin, current, new_version);
        Ok(())
    }

    /// Assigns a role to an address.
    ///
    /// Only callable by the admin. Roles include:
    /// - `admin`: Full admin access
    /// - `fee_manager`: Can manage fees
    /// - `pause_controller`: Can pause/unpause
    pub fn assign_role(
        env: Env,
        admin: Address,
        address: Address,
        role: soroban_sdk::String,
    ) -> Result<(), EscrowError> {
        Self::require_admin(&env, &admin)?;
        storage::assign_role(&env, &address, &role)?;
        events::emit_role_assigned(&env, &admin, &address, &role);
        Ok(())
    }

    /// Returns whether an address has a specific role.
    pub fn has_role(env: Env, address: Address, role: soroban_sdk::String) -> bool {
        storage::has_role(&env, &address, &role)
    }

    fn validate_token(env: &Env, token: &Address) -> Result<(), EscrowError> {
        let client = token::Client::new(env, token);
        client.balance(&env.current_contract_address());
        Ok(())
    }

    fn require_not_paused(env: &Env) -> Result<(), EscrowError> {
        if storage::is_paused(env) {
            return Err(EscrowError::ContractPaused);
        }
        Ok(())
    }

    fn require_admin(env: &Env, admin: &Address) -> Result<(), EscrowError> {
        admin.require_auth();
        let contract_admin = storage::get_admin(env).ok_or(EscrowError::AdminRequired)?;
        if *admin != contract_admin {
            return Err(EscrowError::UnauthorizedAction);
        }
        Ok(())
    }

    fn calculate_fee(escrow: &Escrow) -> i128 {
        if escrow.fee_percent == 0 {
            return 0;
        }
        escrow.amount * escrow.fee_percent as i128 / 100
    }

    fn push_history(
        env: &Env,
        escrow: &mut Escrow,
        from_status: EscrowStatus,
        actor: &Address,
        amount: i128,
    ) {
        escrow.history.push_back(EscrowEvent {
            from_status,
            to_status: escrow.status.clone(),
            actor: actor.clone(),
            timestamp: env.ledger().timestamp(),
            amount,
        });
    }
}
