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
fn test_cancel_escrow_success() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    c.cancel_escrow(&client, &escrow_id);
    let escrow = c.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Cancelled);
    assert!(escrow.cancelled_at.is_some());
}

#[test]
fn test_cancel_escrow_wrong_client() {
    let (env, client, freelancer, token) = setup();
    let wrong_client = Address::generate(&env);
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    let result = c.try_cancel_escrow(&wrong_client, &escrow_id);
    assert!(result.is_err());
}

#[test]
fn test_cancel_escrow_not_found() {
    let (env, client, _, _) = setup();
    let c = contract(&env);
    let result = c.try_cancel_escrow(&client, &999);
    assert!(result.is_err());
}

#[test]
fn test_cancel_escrow_already_funded() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    c.fund_escrow(&client, &escrow_id);
    let result = c.try_cancel_escrow(&client, &escrow_id);
    assert!(result.is_err());
}

#[test]
fn test_cancel_escrow_already_released() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    c.fund_escrow(&client, &escrow_id);
    c.release(&client, &escrow_id);
    let result = c.try_cancel_escrow(&client, &escrow_id);
    assert!(result.is_err());
}

#[test]
fn test_cancel_escrow_already_refunded() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    c.fund_escrow(&client, &escrow_id);
    c.refund(&client, &escrow_id);
    let result = c.try_cancel_escrow(&client, &escrow_id);
    assert!(result.is_err());
}

#[test]
fn test_cancel_escrow_already_cancelled() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    c.cancel_escrow(&client, &escrow_id);
    let result = c.try_cancel_escrow(&client, &escrow_id);
    assert!(result.is_err());
}

#[test]
fn test_cancel_escrow_sets_timestamp() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    c.cancel_escrow(&client, &escrow_id);
    let escrow = c.get_escrow(&escrow_id);
    assert!(escrow.cancelled_at.is_some());
}

#[test]
fn test_cancel_escrow_tracks_history() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    c.cancel_escrow(&client, &escrow_id);
    let history = c.get_history(&escrow_id);
    assert!(history.len() >= 1);
}

#[test]
fn test_cancel_escrow_cannot_fund_after_cancel() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    c.cancel_escrow(&client, &escrow_id);
    let result = c.try_fund_escrow(&client, &escrow_id);
    assert!(result.is_err());
}

#[test]
fn test_cancel_escrow_preserves_details() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &5000, &None);
    c.cancel_escrow(&client, &escrow_id);
    let escrow = c.get_escrow(&escrow_id);
    assert_eq!(escrow.client, client);
    assert_eq!(escrow.freelancer, freelancer);
    assert_eq!(escrow.token, token);
    assert_eq!(escrow.amount, 5000);
}
