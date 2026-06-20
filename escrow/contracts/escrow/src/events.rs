use soroban_sdk::{symbol_short, Address, Env, Symbol};

const ESCROW_CREATED: Symbol = symbol_short!("ESC_CRT");
const ESCROW_FUNDED: Symbol = symbol_short!("ESC_FND");
const ESCROW_RELEASED: Symbol = symbol_short!("ESC_REL");
const ESCROW_REFUNDED: Symbol = symbol_short!("ESC_RFD");

pub(crate) fn emit_escrow_created(
    env: &Env,
    escrow_id: u64,
    client: &Address,
    freelancer: &Address,
    amount: i128,
) {
    env.events().publish(
        (ESCROW_CREATED, escrow_id),
        (client.clone(), freelancer.clone(), amount),
    );
}

pub(crate) fn emit_escrow_funded(env: &Env, escrow_id: u64, amount: i128) {
    env.events()
        .publish((ESCROW_FUNDED, escrow_id), amount);
}

pub(crate) fn emit_escrow_released(env: &Env, escrow_id: u64, amount: i128) {
    env.events()
        .publish((ESCROW_RELEASED, escrow_id), amount);
}

pub(crate) fn emit_escrow_refunded(env: &Env, escrow_id: u64, amount: i128) {
    env.events()
        .publish((ESCROW_REFUNDED, escrow_id), amount);
}
