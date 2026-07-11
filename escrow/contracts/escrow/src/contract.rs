use soroban_sdk::{contract, contractimpl, symbol_short, token, Address, Env, Vec};

use crate::{
    errors::EscrowError,
    events,
    storage,
    types::{Escrow, EscrowEvent, EscrowStatus, Milestone, MilestoneStatus},
};

const MAX_FEE_PERCENT: u32 = 10;

#[contract]
pub struct EscrowContract;

#[contractimpl]
impl EscrowContract {
    pub fn create_escrow(
        env: Env,
        client: Address,
        freelancer: Address,
        token: Address,
        amount: i128,
        deadline: Option<u64>,
    ) -> Result<u64, EscrowError> {
        Self::require_not_paused(&env)?;
        if amount <= 0 {
            return Err(EscrowError::InvalidAmount);
        }
        if client == freelancer {
            return Err(EscrowError::Unauthorized);
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
        if amount <= 0 {
            return Err(EscrowError::InvalidAmount);
        }
        if client == freelancer {
            return Err(EscrowError::Unauthorized);
        }
        if milestone_descriptions.len() != milestone_amounts.len() {
            return Err(EscrowError::InvalidAmount);
        }
        if let Some(dl) = deadline {
            if dl <= env.ledger().timestamp() {
                return Err(EscrowError::DeadlineInPast);
            }
        }
        let total_milestone_amount: i128 = milestone_amounts.iter().sum();
        if total_milestone_amount != amount {
            return Err(EscrowError::InvalidAmount);
        }
        client.require_auth();
        let escrow_id = storage::next_escrow_id(&env);
        let created_at = env.ledger().timestamp();
        let mut milestones = Vec::new(&env);
        for i in 0..milestone_descriptions.len() {
            milestones.push_back(Milestone {
                milestone_id: i as u32,
                description: milestone_descriptions.get(i).unwrap(),
                amount: milestone_amounts.get(i).unwrap(),
                status: MilestoneStatus::Pending,
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

    pub fn fund_escrow(env: Env, client: Address, escrow_id: u64) -> Result<(), EscrowError> {
        Self::require_not_paused(&env)?;
        client.require_auth();
        let mut escrow = storage::get_escrow(&env, escrow_id)?;
        if escrow.client != client {
            return Err(EscrowError::Unauthorized);
        }
        match escrow.status {
            EscrowStatus::Pending => {}
            EscrowStatus::Funded => return Err(EscrowError::AlreadyFunded),
            _ => return Err(EscrowError::InvalidStatus),
        }
        let token_client = token::Client::new(&env, &escrow.token);
        let escrow_amount = escrow.amount;
        token_client.transfer(&client, &env.current_contract_address(), &escrow_amount);
        escrow.status = EscrowStatus::Funded;
        escrow.funded_at = Some(env.ledger().timestamp());
        Self::push_history(&env, &mut escrow, EscrowStatus::Pending, &client, escrow_amount);
        storage::save_escrow(&env, &escrow);
        events::emit_escrow_funded(&env, escrow_id, escrow_amount);
        Ok(())
    }

    pub fn release(env: Env, client: Address, escrow_id: u64) -> Result<(), EscrowError> {
        Self::require_not_paused(&env)?;
        client.require_auth();
        let mut escrow = storage::get_escrow(&env, escrow_id)?;
        if escrow.client != client {
            return Err(EscrowError::Unauthorized);
        }
        match escrow.status {
            EscrowStatus::Funded => {}
            _ => return Err(EscrowError::InvalidStatus),
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
                token_client.transfer(
                    &env.current_contract_address(),
                    &treasury,
                    &fee,
                );
                events::emit_fee_collected(&env, escrow_id, fee, &treasury);
            }
        }
        let old_status = escrow.status.clone();
        escrow.status = EscrowStatus::Released;
        escrow.released_at = Some(env.ledger().timestamp());
        escrow.total_released = release_amount;
        Self::push_history(&env, &mut escrow, old_status, &client, release_amount);
        storage::save_escrow(&env, &escrow);
        events::emit_escrow_released(&env, escrow_id, release_amount);
        Ok(())
    }

    pub fn refund(env: Env, client: Address, escrow_id: u64) -> Result<(), EscrowError> {
        Self::require_not_paused(&env)?;
        client.require_auth();
        let mut escrow = storage::get_escrow(&env, escrow_id)?;
        if escrow.client != client {
            return Err(EscrowError::Unauthorized);
        }
        match escrow.status {
            EscrowStatus::Funded => {}
            _ => return Err(EscrowError::InvalidStatus),
        }
        let token_client = token::Client::new(&env, &escrow.token);
        let refund_amount = escrow.amount;
        token_client.transfer(
            &env.current_contract_address(),
            &client,
            &refund_amount,
        );
        let old_status = escrow.status.clone();
        escrow.status = EscrowStatus::Refunded;
        escrow.refunded_at = Some(env.ledger().timestamp());
        escrow.total_refunded = refund_amount;
        Self::push_history(&env, &mut escrow, old_status, &client, refund_amount);
        storage::save_escrow(&env, &escrow);
        events::emit_escrow_refunded(&env, escrow_id, refund_amount);
        Ok(())
    }

    pub fn cancel_escrow(env: Env, client: Address, escrow_id: u64) -> Result<(), EscrowError> {
        Self::require_not_paused(&env)?;
        client.require_auth();
        let mut escrow = storage::get_escrow(&env, escrow_id)?;
        if escrow.client != client {
            return Err(EscrowError::Unauthorized);
        }
        match escrow.status {
            EscrowStatus::Pending => {}
            _ => return Err(EscrowError::InvalidStatus),
        }
        let old_status = escrow.status.clone();
        escrow.status = EscrowStatus::Cancelled;
        escrow.cancelled_at = Some(env.ledger().timestamp());
        Self::push_history(&env, &mut escrow, old_status, &client, 0);
        storage::save_escrow(&env, &escrow);
        events::emit_escrow_cancelled(&env, escrow_id);
        Ok(())
    }

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
            return Err(EscrowError::Unauthorized);
        }
        match escrow.status {
            EscrowStatus::Pending => {}
            _ => return Err(EscrowError::CannotModifyFundedEscrow),
        }
        if let Some(freelancer) = new_freelancer {
            if client == freelancer {
                return Err(EscrowError::Unauthorized);
            }
            escrow.freelancer = freelancer;
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
        events::emit_escrow_modified(&env, escrow_id, &client);
        Ok(())
    }

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
            return Err(EscrowError::Unauthorized);
        }
        match escrow.status {
            EscrowStatus::Pending | EscrowStatus::Funded => {}
            _ => return Err(EscrowError::InvalidStatus),
        }
        if deadline <= env.ledger().timestamp() {
            return Err(EscrowError::DeadlineInPast);
        }
        escrow.deadline = Some(deadline);
        storage::save_escrow(&env, &escrow);
        Ok(())
    }

    pub fn claim_timeout(env: Env, client: Address, escrow_id: u64) -> Result<(), EscrowError> {
        Self::require_not_paused(&env)?;
        client.require_auth();
        let mut escrow = storage::get_escrow(&env, escrow_id)?;
        if escrow.client != client {
            return Err(EscrowError::Unauthorized);
        }
        if escrow.status != EscrowStatus::Funded {
            return Err(EscrowError::InvalidStatus);
        }
        let deadline = escrow.deadline.ok_or(EscrowError::DeadlineNotPassed)?;
        if env.ledger().timestamp() < deadline {
            return Err(EscrowError::DeadlineNotPassed);
        }
        let token_client = token::Client::new(&env, &escrow.token);
        let claim_amount = escrow.amount;
        token_client.transfer(
            &env.current_contract_address(),
            &client,
            &claim_amount,
        );
        let old_status = escrow.status.clone();
        escrow.status = EscrowStatus::Refunded;
        escrow.refunded_at = Some(env.ledger().timestamp());
        escrow.total_refunded = claim_amount;
        Self::push_history(&env, &mut escrow, old_status, &client, claim_amount);
        storage::save_escrow(&env, &escrow);
        events::emit_escrow_refunded(&env, escrow_id, claim_amount);
        Ok(())
    }

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
            return Err(EscrowError::Unauthorized);
        }
        if escrow.status != EscrowStatus::Funded {
            return Err(EscrowError::InvalidStatus);
        }
        let mut milestones = escrow.milestones.clone();
        let mut found = false;
        for i in 0..milestones.len() {
            let mut m = milestones.get(i).unwrap();
            if m.milestone_id == milestone_id {
                if m.status != MilestoneStatus::Pending && m.status != MilestoneStatus::Submitted {
                    return Err(EscrowError::InvalidStateTransition);
                }
                m.status = MilestoneStatus::Approved;
                milestones.set(i, m);
                found = true;
                events::emit_milestone_approved(&env, escrow_id, milestone_id);
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
            return Err(EscrowError::Unauthorized);
        }
        if escrow.status != EscrowStatus::Funded {
            return Err(EscrowError::InvalidStatus);
        }
        let mut milestones = escrow.milestones.clone();
        let mut found = false;
        for i in 0..milestones.len() {
            let mut m = milestones.get(i).unwrap();
            if m.milestone_id == milestone_id {
                if m.status != MilestoneStatus::Pending && m.status != MilestoneStatus::Submitted {
                    return Err(EscrowError::InvalidStateTransition);
                }
                m.status = MilestoneStatus::Rejected;
                milestones.set(i, m);
                found = true;
                events::emit_milestone_rejected(&env, escrow_id, milestone_id);
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

    pub fn raise_dispute(env: Env, caller: Address, escrow_id: u64) -> Result<(), EscrowError> {
        Self::require_not_paused(&env)?;
        caller.require_auth();
        let mut escrow = storage::get_escrow(&env, escrow_id)?;
        if escrow.client != caller && escrow.freelancer != caller {
            return Err(EscrowError::Unauthorized);
        }
        if escrow.status != EscrowStatus::Funded {
            return Err(EscrowError::InvalidStatus);
        }
        if escrow.disputed_at.is_some() {
            return Err(EscrowError::DisputeAlreadyRaised);
        }
        let old_status = escrow.status.clone();
        escrow.status = EscrowStatus::Disputed;
        escrow.disputed_at = Some(env.ledger().timestamp());
        Self::push_history(&env, &mut escrow, old_status, &caller, 0);
        storage::save_escrow(&env, &escrow);
        events::emit_escrow_disputed(&env, escrow_id, &caller);
        Ok(())
    }

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
            return Err(EscrowError::Unauthorized);
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
            escrow.status = EscrowStatus::Released;
            escrow.released_at = Some(env.ledger().timestamp());
            escrow.total_released = net_freelancer;
            escrow.total_refunded = client_amount;
            events::emit_escrow_resolved(&env, escrow_id, &resolver, "split_funds");
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
            escrow.status = EscrowStatus::Released;
            escrow.released_at = Some(env.ledger().timestamp());
            escrow.total_released = release_amount;
            events::emit_escrow_resolved(&env, escrow_id, &resolver, "released_to_freelancer");
        } else {
            token_client.transfer(
                &env.current_contract_address(),
                &escrow_client,
                &escrow_amount,
            );
            escrow.status = EscrowStatus::Refunded;
            escrow.refunded_at = Some(env.ledger().timestamp());
            escrow.total_refunded = escrow_amount;
            events::emit_escrow_resolved(&env, escrow_id, &resolver, "refunded_to_client");
        }
        storage::save_escrow(&env, &escrow);
        Ok(())
    }

    pub fn set_arbiter(env: Env, admin: Address, escrow_id: u64, arbiter: Address) -> Result<(), EscrowError> {
        Self::require_admin(&env, &admin)?;
        let mut escrow = storage::get_escrow(&env, escrow_id)?;
        escrow.arbiter = Some(arbiter);
        storage::save_escrow(&env, &escrow);
        Ok(())
    }

    pub fn set_fee(
        env: Env,
        admin: Address,
        escrow_id: u64,
        fee_percent: u32,
    ) -> Result<(), EscrowError> {
        Self::require_admin(&env, &admin)?;
        if fee_percent > MAX_FEE_PERCENT {
            return Err(EscrowError::InvalidFeePercentage);
        }
        let mut escrow = storage::get_escrow(&env, escrow_id)?;
        escrow.fee_percent = fee_percent;
        storage::save_escrow(&env, &escrow);
        Ok(())
    }

    pub fn set_default_fee(env: Env, admin: Address, fee_percent: u32) -> Result<(), EscrowError> {
        Self::require_admin(&env, &admin)?;
        if fee_percent > MAX_FEE_PERCENT {
            return Err(EscrowError::InvalidFeePercentage);
        }
        storage::set_default_fee_percent(&env, fee_percent);
        Ok(())
    }

    pub fn set_treasury(env: Env, admin: Address, treasury: Address) -> Result<(), EscrowError> {
        Self::require_admin(&env, &admin)?;
        storage::set_treasury(&env, &treasury);
        Ok(())
    }

    pub fn initialize_admin(env: Env, admin: Address) -> Result<(), EscrowError> {
        if storage::get_admin(&env).is_some() {
            return Err(EscrowError::Unauthorized);
        }
        storage::set_admin(&env, &admin);
        Ok(())
    }

    pub fn set_paused(env: Env, admin: Address, paused: bool) -> Result<(), EscrowError> {
        Self::require_admin(&env, &admin)?;
        storage::set_paused(&env, paused);
        Ok(())
    }

    pub fn get_escrow(env: Env, escrow_id: u64) -> Result<Escrow, EscrowError> {
        storage::get_escrow(&env, escrow_id)
    }

    pub fn get_history(env: Env, escrow_id: u64) -> Result<Vec<EscrowEvent>, EscrowError> {
        let escrow = storage::get_escrow(&env, escrow_id)?;
        Ok(escrow.history)
    }

    pub fn get_admin(env: Env) -> Option<Address> {
        storage::get_admin(&env)
    }

    pub fn is_paused(env: Env) -> bool {
        storage::is_paused(&env)
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
            return Err(EscrowError::Unauthorized);
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
