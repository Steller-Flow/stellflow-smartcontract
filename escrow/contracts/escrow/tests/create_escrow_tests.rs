#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, Env};
use stellflow_escrow::{EscrowContract, EscrowStatus};

fn setup() -> (Env, EscrowContract, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract = EscrowContract::new(&env);
    let client = Address::generate(&env);
    let freelancer = Address::generate(&env);
    let token = Address::generate(&env);
    (env, contract, client, freelancer, token)
}

#[test]
fn test_create_escrow_success() {
    let (env, contract, client, freelancer, token) = setup();
    let escrow_id = contract.create_escrow(client.clone(), freelancer.clone(), token.clone(), 1000);
    let escrow = contract.get_escrow(escrow_id);
    assert_eq!(escrow.escrow_id, 1);
    assert_eq!(escrow.client, client);
    assert_eq!(escrow.freelancer, freelancer);
    assert_eq!(escrow.token, token);
    assert_eq!(escrow.amount, 1000);
    assert_eq!(escrow.status, EscrowStatus::Pending);
    assert!(escrow.funded_at.is_none());
    assert!(escrow.released_at.is_none());
    assert!(escrow.refunded_at.is_none());
}

#[test]
fn test_create_escrow_zero_amount() {
    let (env, contract, client, freelancer, token) = setup();
    let result = contract.try_create_escrow(client, freelancer, token, 0);
    assert!(result.is_err());
}

#[test]
fn test_create_escrow_negative_amount() {
    let (env, contract, client, freelancer, token) = setup();
    let result = contract.try_create_escrow(client, freelancer, token, -100);
    assert!(result.is_err());
}

#[test]
fn test_create_escrow_same_client_and_freelancer() {
    let (env, contract, client, _, token) = setup();
    let result = contract.try_create_escrow(client.clone(), client, token, 1000);
    assert!(result.is_err());
}

#[test]
fn test_create_escrow_sequential_ids() {
    let (env, contract, client, freelancer, token) = setup();
    let id1 = contract.create_escrow(client.clone(), freelancer.clone(), token.clone(), 1000);
    let id2 = contract.create_escrow(client.clone(), freelancer.clone(), token.clone(), 2000);
    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
}
