use soroban_sdk::Env;

use crate::{
    errors::EscrowError,
    types::{DataKey, Escrow},
};

// TTL: ~1 year worth of ledgers (roughly 2 000 000 at 5-second close time)
const ESCROW_TTL: u32 = 2_000_000;

// ============================================================
// COUNTER — generates sequential escrow IDs
// ============================================================

/// Read the current counter value (0 if not yet set).
pub fn read_counter(env: &Env) -> u64 {
    env.storage()
        .instance()
        .get::<DataKey, u64>(&DataKey::EscrowCounter)
        .unwrap_or(0)
}

/// Increment the counter and return the *new* value as the next escrow ID.
pub fn next_escrow_id(env: &Env) -> u64 {
    let next = read_counter(env) + 1;
    env.storage()
        .instance()
        .set(&DataKey::EscrowCounter, &next);
    // Extend instance TTL so the counter survives long-running deployments
    env.storage().instance().extend_ttl(ESCROW_TTL, ESCROW_TTL);
    next
}

// ============================================================
// ESCROW CRUD
// ============================================================

/// Persist an escrow to persistent storage.
pub fn save_escrow(env: &Env, escrow: &Escrow) {
    let key = DataKey::Escrow(escrow.escrow_id);
    env.storage().persistent().set(&key, escrow);
    env.storage()
        .persistent()
        .extend_ttl(&key, ESCROW_TTL, ESCROW_TTL);
}

/// Retrieve an escrow by ID, returning `EscrowError::EscrowNotFound` if absent.
pub fn get_escrow(env: &Env, escrow_id: u64) -> Result<Escrow, EscrowError> {
    env.storage()
        .persistent()
        .get::<DataKey, Escrow>(&DataKey::Escrow(escrow_id))
        .ok_or(EscrowError::EscrowNotFound)
}

/// Return true if an escrow exists for the given ID.
pub fn escrow_exists(env: &Env, escrow_id: u64) -> bool {
    env.storage()
        .persistent()
        .has(&DataKey::Escrow(escrow_id))
}