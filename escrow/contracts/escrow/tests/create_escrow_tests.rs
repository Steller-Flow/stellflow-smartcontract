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
fn test_create_escrow_success() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &1000, &None);
    let escrow = c.get_escrow(&escrow_id);
    assert_eq!(escrow.escrow_id, 1);
    assert_eq!(escrow.client, client);
    assert_eq!(escrow.freelancer, freelancer);
    assert_eq!(escrow.token, token);
    assert_eq!(escrow.amount, 1000);
    assert_eq!(escrow.status, EscrowStatus::Pending);
    assert!(escrow.milestones.is_empty());
}

#[test]
fn test_create_escrow_zero_amount() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let result = c.try_create_escrow(&client, &freelancer, &token, &0, &None);
    assert!(result.is_err());
}

#[test]
fn test_create_escrow_negative_amount() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let result = c.try_create_escrow(&client, &freelancer, &token, &-100, &None);
    assert!(result.is_err());
}

#[test]
fn test_create_escrow_same_client_and_freelancer() {
    let (env, client, _, token) = setup();
    let c = contract(&env);
    let result = c.try_create_escrow(&client, &client, &token, &1000, &None);
    assert!(result.is_err());
}

#[test]
fn test_create_escrow_with_deadline() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &1000, &Some(200));
    let escrow = c.get_escrow(&escrow_id);
    assert_eq!(escrow.deadline, Some(200));
}

#[test]
fn test_create_escrow_deadline_in_past() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let result = c.try_create_escrow(&client, &freelancer, &token, &1000, &Some(0));
    assert!(result.is_err());
}

#[test]
fn test_create_escrow_sequential_ids() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let id1 = c.create_escrow(&client, &freelancer, &token, &1000, &None);
    let id2 = c.create_escrow(&client, &freelancer, &token, &2000, &None);
    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
}
