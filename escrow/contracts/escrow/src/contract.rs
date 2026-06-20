use soroban_sdk::{contract, contractimpl, token, Address, Env};

use crate::{
    errors::EscrowError,
    events,
    storage,
    types::{Escrow, EscrowStatus},
};

#[contract]
pub struct EscrowContract;

#[contractimpl]
impl EscrowContract {
    /// Creates a new escrow, returns escrow_id, status = Pending.
    /// Auth: client must authorise.
    /// Validation: amount > 0, client != freelancer.
    pub fn create_escrow(
        env: Env,
        client: Address,
        freelancer: Address,
        token: Address,
        amount: i128,
    ) -> Result<u64, EscrowError> {
        if amount <= 0 {
            return Err(EscrowError::InvalidAmount);
        }
        if client == freelancer {
            return Err(EscrowError::Unauthorized);
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
        };

        storage::save_escrow(&env, &escrow);
        events::emit_escrow_created(&env, escrow_id, &client, &freelancer, amount);

        Ok(escrow_id)
    }

    /// Funds the escrow. Caller must be client, status must be Pending.
    /// Transfers tokens: client → contract. Updates: status = Funded.
    pub fn fund_escrow(
        env: Env,
        client: Address,
        escrow_id: u64,
    ) -> Result<(), EscrowError> {
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
        token_client.transfer(&client, &env.current_contract_address(), &escrow.amount);

        escrow.status = EscrowStatus::Funded;
        escrow.funded_at = Some(env.ledger().timestamp());

        storage::save_escrow(&env, &escrow);
        events::emit_escrow_funded(&env, escrow_id, escrow.amount);

        Ok(())
    }

    /// Releases escrow funds to freelancer. Caller must be client, status must be Funded.
    /// Transfers tokens: contract → freelancer. Updates: status = Released.
    pub fn release(
        env: Env,
        client: Address,
        escrow_id: u64,
    ) -> Result<(), EscrowError> {
        client.require_auth();

        let mut escrow = storage::get_escrow(&env, escrow_id)?;

        if escrow.client != client {
            return Err(EscrowError::Unauthorized);
        }

        match escrow.status {
            EscrowStatus::Funded => {}
            EscrowStatus::Released => return Err(EscrowError::AlreadyReleased),
            EscrowStatus::Refunded => return Err(EscrowError::AlreadyRefunded),
            _ => return Err(EscrowError::InvalidStatus),
        }

        let token_client = token::Client::new(&env, &escrow.token);
        token_client.transfer(
            &env.current_contract_address(),
            &escrow.freelancer,
            &escrow.amount,
        );

        escrow.status = EscrowStatus::Released;
        escrow.released_at = Some(env.ledger().timestamp());

        storage::save_escrow(&env, &escrow);
        events::emit_escrow_released(&env, escrow_id, escrow.amount);

        Ok(())
    }

    /// Refunds escrow funds to client. Caller must be client, status must be Funded.
    /// Transfers tokens: contract → client. Updates: status = Refunded.
    pub fn refund(
        env: Env,
        client: Address,
        escrow_id: u64,
    ) -> Result<(), EscrowError> {
        client.require_auth();

        let mut escrow = storage::get_escrow(&env, escrow_id)?;

        if escrow.client != client {
            return Err(EscrowError::Unauthorized);
        }

        match escrow.status {
            EscrowStatus::Funded => {}
            EscrowStatus::Released => return Err(EscrowError::AlreadyReleased),
            EscrowStatus::Refunded => return Err(EscrowError::AlreadyRefunded),
            _ => return Err(EscrowError::InvalidStatus),
        }

        let token_client = token::Client::new(&env, &escrow.token);
        token_client.transfer(
            &env.current_contract_address(),
            &client,
            &escrow.amount,
        );

        escrow.status = EscrowStatus::Refunded;
        escrow.refunded_at = Some(env.ledger().timestamp());

        storage::save_escrow(&env, &escrow);
        events::emit_escrow_refunded(&env, escrow_id, escrow.amount);

        Ok(())
    }

    /// Returns the escrow details for the given id.
    pub fn get_escrow(env: Env, escrow_id: u64) -> Result<Escrow, EscrowError> {
        storage::get_escrow(&env, escrow_id)
    }
}
