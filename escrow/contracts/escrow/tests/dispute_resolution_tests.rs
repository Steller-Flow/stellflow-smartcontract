#![allow(deprecated)]
#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, Env};
use stellflow_escrow::{EscrowContract, EscrowStatus};

fn setup() -> (Env, Address, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let client = Address::generate(&env);
    let freelancer = Address::generate(&env);
    let admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(Address::generate(&env));
    let token = token_contract.address();
    let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token);
    token_client.mint(&client, &100_000_000);
    (env, client, freelancer, admin, token)
}

fn contract(env: &Env) -> stellflow_escrow::contract::EscrowContractClient<'_> {
    let contract_id = env.register_contract(None, EscrowContract);
    stellflow_escrow::contract::EscrowContractClient::new(env, &contract_id)
}

#[test]
fn test_dispute_resolution_release_to_freelancer() {
    let (env, client, freelancer, admin, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    c.fund_escrow(&client, &escrow_id);
    c.raise_dispute(&freelancer, &escrow_id);
    let escrow = c.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Disputed);
    c.initialize_admin(&admin);
    c.resolve_dispute(&admin, &escrow_id, &true, &None);
    let escrow = c.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Released);
    assert_eq!(escrow.total_released, 10000);
    assert!(escrow.released_at.is_some());
}

#[test]
fn test_dispute_resolution_refund_to_client() {
    let (env, client, freelancer, admin, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    c.fund_escrow(&client, &escrow_id);
    c.raise_dispute(&client, &escrow_id);
    c.initialize_admin(&admin);
    c.resolve_dispute(&admin, &escrow_id, &false, &None);
    let escrow = c.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Refunded);
    assert_eq!(escrow.total_refunded, 10000);
    assert!(escrow.refunded_at.is_some());
}

#[test]
fn test_dispute_resolution_split_funds() {
    let (env, client, freelancer, admin, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    c.fund_escrow(&client, &escrow_id);
    c.raise_dispute(&freelancer, &escrow_id);
    c.initialize_admin(&admin);
    c.resolve_dispute(&admin, &escrow_id, &false, &Some(6000));
    let escrow = c.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Released);
    assert_eq!(escrow.total_released, 6000);
    assert_eq!(escrow.total_refunded, 4000);
    assert!(escrow.released_at.is_some());
}

#[test]
fn test_dispute_resolution_split_all_to_client() {
    let (env, client, freelancer, admin, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    c.fund_escrow(&client, &escrow_id);
    c.raise_dispute(&freelancer, &escrow_id);
    c.initialize_admin(&admin);
    c.resolve_dispute(&admin, &escrow_id, &false, &Some(0));
    let escrow = c.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Released);
    assert_eq!(escrow.total_released, 0);
    assert_eq!(escrow.total_refunded, 10000);
}

#[test]
fn test_dispute_resolution_split_all_to_freelancer() {
    let (env, client, freelancer, admin, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    c.fund_escrow(&client, &escrow_id);
    c.raise_dispute(&freelancer, &escrow_id);
    c.initialize_admin(&admin);
    c.resolve_dispute(&admin, &escrow_id, &false, &Some(10000));
    let escrow = c.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Released);
    assert_eq!(escrow.total_released, 10000);
    assert_eq!(escrow.total_refunded, 0);
}

#[test]
fn test_dispute_split_invalid_amount() {
    let (env, client, freelancer, admin, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    c.fund_escrow(&client, &escrow_id);
    c.raise_dispute(&freelancer, &escrow_id);
    c.initialize_admin(&admin);
    let result = c.try_resolve_dispute(&admin, &escrow_id, &false, &Some(15000));
    assert!(result.is_err());
}

#[test]
fn test_dispute_split_negative_amount() {
    let (env, client, freelancer, admin, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    c.fund_escrow(&client, &escrow_id);
    c.raise_dispute(&freelancer, &escrow_id);
    c.initialize_admin(&admin);
    let result = c.try_resolve_dispute(&admin, &escrow_id, &false, &Some(-1000));
    assert!(result.is_err());
}

#[test]
fn test_raise_dispute_by_client() {
    let (env, client, freelancer, _, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    c.fund_escrow(&client, &escrow_id);
    c.raise_dispute(&client, &escrow_id);
    let escrow = c.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Disputed);
    assert!(escrow.disputed_at.is_some());
}

#[test]
fn test_raise_dispute_by_freelancer() {
    let (env, client, freelancer, _, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    c.fund_escrow(&client, &escrow_id);
    c.raise_dispute(&freelancer, &escrow_id);
    let escrow = c.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Disputed);
}

#[test]
fn test_raise_dispute_unauthorized() {
    let (env, client, freelancer, _, token) = setup();
    let wrong_person = Address::generate(&env);
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    c.fund_escrow(&client, &escrow_id);
    let result = c.try_raise_dispute(&wrong_person, &escrow_id);
    assert!(result.is_err());
}

#[test]
fn test_raise_dispute_not_funded() {
    let (env, client, freelancer, _, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    let result = c.try_raise_dispute(&client, &escrow_id);
    assert!(result.is_err());
}

#[test]
fn test_raise_dispute_already_disputed() {
    let (env, client, freelancer, _, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    c.fund_escrow(&client, &escrow_id);
    c.raise_dispute(&client, &escrow_id);
    let result = c.try_raise_dispute(&freelancer, &escrow_id);
    assert!(result.is_err());
}

#[test]
fn test_resolve_dispute_unauthorized() {
    let (env, client, freelancer, admin, token) = setup();
    let wrong_admin = Address::generate(&env);
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    c.fund_escrow(&client, &escrow_id);
    c.raise_dispute(&client, &escrow_id);
    c.initialize_admin(&admin);
    let result = c.try_resolve_dispute(&wrong_admin, &escrow_id, &true, &None);
    assert!(result.is_err());
}

#[test]
fn test_resolve_dispute_not_disputed() {
    let (env, client, freelancer, admin, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    c.fund_escrow(&client, &escrow_id);
    c.initialize_admin(&admin);
    let result = c.try_resolve_dispute(&admin, &escrow_id, &true, &None);
    assert!(result.is_err());
}

#[test]
fn test_dispute_funds_locked_during_dispute() {
    let (env, client, freelancer, _, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    c.fund_escrow(&client, &escrow_id);
    c.raise_dispute(&client, &escrow_id);
    let result = c.try_release(&client, &escrow_id);
    assert!(result.is_err());
    let result = c.try_refund(&client, &escrow_id);
    assert!(result.is_err());
}

#[test]
fn test_dispute_sets_disputed_at_timestamp() {
    let (env, client, freelancer, _, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    c.fund_escrow(&client, &escrow_id);
    c.raise_dispute(&client, &escrow_id);
    let escrow = c.get_escrow(&escrow_id);
    assert!(escrow.disputed_at.is_some());
}
