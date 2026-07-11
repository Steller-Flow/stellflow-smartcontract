use soroban_sdk::{Address, Env};

use crate::{
    errors::EscrowError,
    types::{DataKey, Escrow},
};

const ESCROW_TTL: u32 = 2_000_000;

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
    env.storage().instance().extend_ttl(ESCROW_TTL, ESCROW_TTL);
    next
}

pub fn save_escrow(env: &Env, escrow: &Escrow) {
    let key = DataKey::Escrow(escrow.escrow_id);
    env.storage().persistent().set(&key, escrow);
    env.storage()
        .persistent()
        .extend_ttl(&key, ESCROW_TTL, ESCROW_TTL);
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
