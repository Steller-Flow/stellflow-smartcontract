use soroban_sdk::{symbol_short, Address, Env, Symbol};

const ESCROW_CREATED: Symbol = symbol_short!("ESC_CRT");
const ESCROW_FUNDED: Symbol = symbol_short!("ESC_FND");
const ESCROW_RELEASED: Symbol = symbol_short!("ESC_REL");
const ESCROW_REFUNDED: Symbol = symbol_short!("ESC_RFD");
const ESCROW_CANCELLED: Symbol = symbol_short!("ESC_CAN");
const ESCROW_DISPUTED: Symbol = symbol_short!("ESC_DPT");
const ESCROW_RESOLVED: Symbol = symbol_short!("ESC_RSV");
const ESCROW_MODIFIED: Symbol = symbol_short!("ESC_MDF");
const MILESTONE_APPROVED: Symbol = symbol_short!("MSN_APR");
const MILESTONE_REJECTED: Symbol = symbol_short!("MSN_RJT");
const FEE_COLLECTED: Symbol = symbol_short!("FEE_COL");

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

pub(crate) fn emit_escrow_cancelled(env: &Env, escrow_id: u64) {
    env.events().publish((ESCROW_CANCELLED, escrow_id), ());
}

pub(crate) fn emit_escrow_disputed(env: &Env, escrow_id: u64, raiser: &Address) {
    env.events()
        .publish((ESCROW_DISPUTED, escrow_id), raiser.clone());
}

pub(crate) fn emit_escrow_resolved(
    env: &Env,
    escrow_id: u64,
    resolver: &Address,
    outcome: &str,
) {
    env.events().publish(
        (ESCROW_RESOLVED, escrow_id),
        (resolver.clone(), soroban_sdk::String::from_str(env, outcome)),
    );
}

pub(crate) fn emit_escrow_modified(env: &Env, escrow_id: u64, modifier: &Address) {
    env.events()
        .publish((ESCROW_MODIFIED, escrow_id), modifier.clone());
}

pub(crate) fn emit_milestone_approved(env: &Env, escrow_id: u64, milestone_id: u32) {
    env.events()
        .publish((MILESTONE_APPROVED, escrow_id), milestone_id);
}

pub(crate) fn emit_milestone_rejected(env: &Env, escrow_id: u64, milestone_id: u32) {
    env.events()
        .publish((MILESTONE_REJECTED, escrow_id), milestone_id);
}

pub(crate) fn emit_fee_collected(
    env: &Env,
    escrow_id: u64,
    fee_amount: i128,
    treasury: &Address,
) {
    env.events().publish(
        (FEE_COLLECTED, escrow_id),
        (fee_amount, treasury.clone()),
    );
}
