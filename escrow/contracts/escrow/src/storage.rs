use soroban_sdk::{Address, Env};

use crate::{
    errors::EscrowError,
    types::{DataKey, Escrow},
};

const DEFAULT_ESCROW_TTL: u32 = 2_000_000;
const MIN_TTL: u32 = 1_000_000;
const MAX_TTL: u32 = 7_776_000;

pub fn get_escrow_ttl(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get::<DataKey, u32>(&DataKey::EscrowTTL)
        .unwrap_or(DEFAULT_ESCROW_TTL)
}

pub fn set_escrow_ttl(env: &Env, ttl: u32) -> Result<(), EscrowError> {
    if ttl < MIN_TTL || ttl > MAX_TTL {
        return Err(EscrowError::InvalidAmount);
    }
    env.storage().instance().set(&DataKey::EscrowTTL, &ttl);
    Ok(())
}

pub fn read_counter(env: &Env) -> u64 {
    env.storage()
        .instance()
        .get::<DataKey, u64>(&DataKey::EscrowCounter)
        .unwrap_or(0)
}

pub fn next_escrow_id(env: &Env) -> u64 {
    let next = read_counter(env) + 1;
    env.storage()
        .instance()
        .set(&DataKey::EscrowCounter, &next);
    let ttl = get_escrow_ttl(env);
    env.storage().instance().extend_ttl(ttl, ttl);
    next
}

pub fn save_escrow(env: &Env, escrow: &Escrow) {
    let key = DataKey::Escrow(escrow.escrow_id);
    env.storage().persistent().set(&key, escrow);
    let ttl = get_escrow_ttl(env);
    env.storage()
        .persistent()
        .extend_ttl(&key, ttl, ttl);
}

pub fn get_escrow(env: &Env, escrow_id: u64) -> Result<Escrow, EscrowError> {
    env.storage()
        .persistent()
        .get::<DataKey, Escrow>(&DataKey::Escrow(escrow_id))
        .ok_or(EscrowError::EscrowNotFound)
}

pub fn escrow_exists(env: &Env, escrow_id: u64) -> bool {
    env.storage()
        .persistent()
        .has(&DataKey::Escrow(escrow_id))
}

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
                    env.storage()
                        .persistent()
                        .remove(&DataKey::Escrow(id));
                    cleaned += 1;
                }
            }
        }
    }
    Ok(cleaned)
}

pub fn get_admin(env: &Env) -> Option<Address> {
    env.storage().instance().get::<DataKey, Address>(&DataKey::Admin)
}

pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::Admin, admin);
}

pub fn is_paused(env: &Env) -> bool {
    env.storage()
        .instance()
        .get::<DataKey, bool>(&DataKey::Paused)
        .unwrap_or(false)
}

pub fn set_paused(env: &Env, paused: bool) {
    env.storage().instance().set(&DataKey::Paused, &paused);
}

pub fn get_treasury(env: &Env) -> Option<Address> {
    env.storage()
        .instance()
        .get::<DataKey, Address>(&DataKey::PlatformTreasury)
}

pub fn set_treasury(env: &Env, treasury: &Address) {
    env.storage()
        .instance()
        .set(&DataKey::PlatformTreasury, treasury);
}

pub fn get_default_fee_percent(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get::<DataKey, u32>(&DataKey::DefaultFeePercent)
        .unwrap_or(0)
}

pub fn set_default_fee_percent(env: &Env, percent: u32) {
    env.storage()
        .instance()
        .set(&DataKey::DefaultFeePercent, &percent);
}
