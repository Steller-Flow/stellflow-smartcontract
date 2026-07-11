#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, Env};
use stellflow_escrow::{EscrowContract, EscrowStatus};

fn setup() -> (Env, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let client = Address::generate(&env);
    let freelancer = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(Address::generate(&env));
    let token = token_contract.address();
    let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token);
    token_client.mint(&client, &100_000_000);
    (env, client, freelancer, token)
}

fn contract(env: &Env) -> stellflow_escrow::contract::EscrowContractClient<'_> {
    let contract_id = env.register_contract(None, EscrowContract);
    stellflow_escrow::contract::EscrowContractClient::new(env, &contract_id)
}

#[test]
fn test_release_success() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &1000, &None);
    c.fund_escrow(&client, &escrow_id);
    c.release(&client, &escrow_id);
    let escrow = c.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Released);
    assert!(escrow.released_at.is_some());
    assert_eq!(escrow.total_released, 1000);
}

#[test]
fn test_release_not_funded() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &1000, &None);
    let result = c.try_release(&client, &escrow_id);
    assert!(result.is_err());
}

#[test]
fn test_release_wrong_client() {
    let (env, client, freelancer, token) = setup();
    let wrong_client = Address::generate(&env);
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &1000, &None);
    c.fund_escrow(&client, &escrow_id);
    let result = c.try_release(&wrong_client, &escrow_id);
    assert!(result.is_err());
}

#[test]
fn test_release_already_released() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &1000, &None);
    c.fund_escrow(&client, &escrow_id);
    c.release(&client, &escrow_id);
    let result = c.try_release(&client, &escrow_id);
    assert!(result.is_err());
}

#[test]
fn test_release_already_refunded() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &1000, &None);
    c.fund_escrow(&client, &escrow_id);
    c.refund(&client, &escrow_id);
    let result = c.try_release(&client, &escrow_id);
    assert!(result.is_err());
}

#[test]
fn test_release_sets_released_at_timestamp() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &1000, &None);
    c.fund_escrow(&client, &escrow_id);
    c.release(&client, &escrow_id);
    let escrow = c.get_escrow(&escrow_id);
    assert!(escrow.released_at.is_some());
}

#[test]
fn test_release_tracks_total_released() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &5000, &None);
    c.fund_escrow(&client, &escrow_id);
    c.release(&client, &escrow_id);
    let escrow = c.get_escrow(&escrow_id);
    assert_eq!(escrow.total_released, 5000);
}

#[test]
fn test_refund_success() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &1000, &None);
    c.fund_escrow(&client, &escrow_id);
    c.refund(&client, &escrow_id);
    let escrow = c.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Refunded);
    assert!(escrow.refunded_at.is_some());
    assert_eq!(escrow.total_refunded, 1000);
}

#[test]
fn test_refund_not_funded() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &1000, &None);
    let result = c.try_refund(&client, &escrow_id);
    assert!(result.is_err());
}

#[test]
fn test_refund_wrong_client() {
    let (env, client, freelancer, token) = setup();
    let wrong_client = Address::generate(&env);
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &1000, &None);
    c.fund_escrow(&client, &escrow_id);
    let result = c.try_refund(&wrong_client, &escrow_id);
    assert!(result.is_err());
}

#[test]
fn test_refund_already_refunded() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &1000, &None);
    c.fund_escrow(&client, &escrow_id);
    c.refund(&client, &escrow_id);
    let result = c.try_refund(&client, &escrow_id);
    assert!(result.is_err());
}

#[test]
fn test_refund_already_released() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &1000, &None);
    c.fund_escrow(&client, &escrow_id);
    c.release(&client, &escrow_id);
    let result = c.try_refund(&client, &escrow_id);
    assert!(result.is_err());
}

#[test]
fn test_refund_sets_refunded_at_timestamp() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &1000, &None);
    c.fund_escrow(&client, &escrow_id);
    c.refund(&client, &escrow_id);
    let escrow = c.get_escrow(&escrow_id);
    assert!(escrow.refunded_at.is_some());
}

#[test]
fn test_refund_tracks_total_refunded() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &3000, &None);
    c.fund_escrow(&client, &escrow_id);
    c.refund(&client, &escrow_id);
    let escrow = c.get_escrow(&escrow_id);
    assert_eq!(escrow.total_refunded, 3000);
}
