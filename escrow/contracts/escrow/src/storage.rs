use soroban_sdk::{Address, Env};

use crate::{
    errors::EscrowError,
    types::{DataKey, Escrow},
};

const DEFAULT_ESCROW_TTL: u32 = 2_000_000;
const MIN_TTL: u32 = 1_000_000;
const MAX_TTL: u32 = 7_776_000;
pub const CURRENT_VERSION: u32 = 1;

/// Returns the configured TTL for escrow storage.
///
/// Defaults to `2_000_000` ledger increments if not configured.
/// Can be updated by admin via `set_escrow_ttl`.
pub fn get_escrow_ttl(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get::<DataKey, u32>(&DataKey::EscrowTTL)
        .unwrap_or(DEFAULT_ESCROW_TTL)
}

/// Sets the TTL for escrow storage.
///
/// # Arguments
/// * `ttl` - TTL in ledger increments (must be between 1,000,000 and 7,776,000)
///
/// # Errors
/// Returns `EscrowError::InvalidAmount` if TTL is outside the allowed range.
pub fn set_escrow_ttl(env: &Env, ttl: u32) -> Result<(), EscrowError> {
    if !(MIN_TTL..=MAX_TTL).contains(&ttl) {
        return Err(EscrowError::InvalidAmount);
    }
    env.storage().instance().set(&DataKey::EscrowTTL, &ttl);
    Ok(())
}

/// Reads the current escrow counter value.
///
/// Returns 0 if no escrows have been created yet.
pub fn read_counter(env: &Env) -> u64 {
    env.storage()
        .instance()
        .get::<DataKey, u64>(&DataKey::EscrowCounter)
        .unwrap_or(0)
}

/// Returns the next escrow ID and increments the counter.
///
/// Also extends the instance storage TTL to match the configured escrow TTL.
pub fn next_escrow_id(env: &Env) -> u64 {
    let next = read_counter(env) + 1;
    env.storage().instance().set(&DataKey::EscrowCounter, &next);
    let ttl = get_escrow_ttl(env);
    env.storage().instance().extend_ttl(ttl, ttl);
    next
}

/// Saves an escrow to persistent storage.
///
/// Sets the TTL on the storage entry to match the configured escrow TTL.
pub fn save_escrow(env: &Env, escrow: &Escrow) {
    let key = DataKey::Escrow(escrow.escrow_id);
    env.storage().persistent().set(&key, escrow);
    let ttl = get_escrow_ttl(env);
    env.storage().persistent().extend_ttl(&key, ttl, ttl);
}

/// Retrieves an escrow from persistent storage.
///
/// # Errors
/// Returns `EscrowError::EscrowNotFound` if no escrow exists with the given ID.
pub fn get_escrow(env: &Env, escrow_id: u64) -> Result<Escrow, EscrowError> {
    env.storage()
        .persistent()
        .get::<DataKey, Escrow>(&DataKey::Escrow(escrow_id))
        .ok_or(EscrowError::EscrowNotFound)
}

/// Checks whether an escrow exists in storage.
pub fn escrow_exists(env: &Env, escrow_id: u64) -> bool {
    env.storage().persistent().has(&DataKey::Escrow(escrow_id))
}

/// Cleans up expired terminal escrows from storage.
///
/// Only removes escrows in terminal states (Released, Refunded, Cancelled)
/// that have been in that state longer than the configured TTL.
///
/// # Returns
/// The number of escrows that were cleaned up.
pub fn cleanup_expired_escrows(env: &Env, admin: &Address) -> Result<u32, EscrowError> {
    admin.require_auth();
    let contract_admin = get_admin(env).ok_or(EscrowError::AdminRequired)?;
    if *admin != contract_admin {
        return Err(EscrowError::UnauthorizedAction);
    }
    let counter = read_counter(env);
    let mut cleaned = 0u32;
    let timestamp = env.ledger().timestamp();
    let ttl = get_escrow_ttl(env);
    let ttl_seconds = ttl as u64 * 5;
    for id in 1..=counter {
        if let Some(escrow) = env
            .storage()
            .persistent()
            .get::<DataKey, Escrow>(&DataKey::Escrow(id))
        {
            let is_terminal = matches!(
                escrow.status,
                crate::types::EscrowStatus::Released
                    | crate::types::EscrowStatus::Refunded
                    | crate::types::EscrowStatus::Cancelled
            );
            if is_terminal {
                let terminal_time = escrow
                    .released_at
                    .or(escrow.refunded_at)
                    .or(escrow.cancelled_at)
                    .unwrap_or(0);
                if terminal_time > 0 && timestamp.saturating_sub(terminal_time) > ttl_seconds {
                    env.storage().persistent().remove(&DataKey::Escrow(id));
                    cleaned += 1;
                }
            }
        }
    }
    Ok(cleaned)
}

/// Returns the admin address, or `None` if not initialized.
pub fn get_admin(env: &Env) -> Option<Address> {
    env.storage()
        .instance()
        .get::<DataKey, Address>(&DataKey::Admin)
}

/// Sets the admin address in instance storage.
pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::Admin, admin);
}

/// Returns whether the contract is paused.
pub fn is_paused(env: &Env) -> bool {
    env.storage()
        .instance()
        .get::<DataKey, bool>(&DataKey::Paused)
        .unwrap_or(false)
}

/// Sets the paused state of the contract.
pub fn set_paused(env: &Env, paused: bool) {
    env.storage().instance().set(&DataKey::Paused, &paused);
}

/// Returns the treasury address, or `None` if not configured.
pub fn get_treasury(env: &Env) -> Option<Address> {
    env.storage()
        .instance()
        .get::<DataKey, Address>(&DataKey::PlatformTreasury)
}

/// Sets the treasury address for fee collection.
pub fn set_treasury(env: &Env, treasury: &Address) {
    env.storage()
        .instance()
        .set(&DataKey::PlatformTreasury, treasury);
}

/// Returns the default fee percentage for new escrows.
///
/// Defaults to 0 (no fee) if not configured.
pub fn get_default_fee_percent(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get::<DataKey, u32>(&DataKey::DefaultFeePercent)
        .unwrap_or(0)
}

/// Sets the default fee percentage for new escrows.
pub fn set_default_fee_percent(env: &Env, percent: u32) {
    env.storage()
        .instance()
        .set(&DataKey::DefaultFeePercent, &percent);
}

/// Returns the current contract version.
///
/// Defaults to 0 if not set (pre-versioned contracts).
pub fn get_version(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get::<DataKey, u32>(&DataKey::Version)
        .unwrap_or(0)
}

/// Sets the contract version number.
pub fn set_version(env: &Env, version: u32) {
    env.storage().instance().set(&DataKey::Version, &version);
}

/// Checks if an address has a specific role.
pub fn has_role(env: &Env, address: &Address, role: &soroban_sdk::String) -> bool {
    let key = DataKey::Role(address.clone());
    if let Some(roles) = env
        .storage()
        .instance()
        .get::<DataKey, soroban_sdk::Vec<soroban_sdk::String>>(&key)
    {
        for i in 0..roles.len() {
            if roles.get(i).unwrap() == *role {
                return true;
            }
        }
    }
    false
}

/// Assigns a role to an address.
pub fn assign_role(
    env: &Env,
    address: &Address,
    role: &soroban_sdk::String,
) -> Result<(), EscrowError> {
    let key = DataKey::Role(address.clone());
    let mut roles = env
        .storage()
        .instance()
        .get::<DataKey, soroban_sdk::Vec<soroban_sdk::String>>(&key)
        .unwrap_or(soroban_sdk::Vec::new(env));
    for i in 0..roles.len() {
        if roles.get(i).unwrap() == *role {
            return Err(EscrowError::RoleAlreadyAssigned);
        }
    }
    roles.push_back(role.clone());
    env.storage().instance().set(&key, &roles);
    Ok(())
}

/// Removes a role from an address.
pub fn remove_role(
    env: &Env,
    address: &Address,
    role: &soroban_sdk::String,
) -> Result<(), EscrowError> {
    let key = DataKey::Role(address.clone());
    let roles = env
        .storage()
        .instance()
        .get::<DataKey, soroban_sdk::Vec<soroban_sdk::String>>(&key)
        .unwrap_or(soroban_sdk::Vec::new(env));
    let mut new_roles = soroban_sdk::Vec::new(env);
    let mut found = false;
    for i in 0..roles.len() {
        let r = roles.get(i).unwrap();
        if r == *role {
            found = true;
        } else {
            new_roles.push_back(r);
        }
    }
    if !found {
        return Err(EscrowError::RoleNotFound);
    }
    env.storage().instance().set(&key, &new_roles);
    Ok(())
}
