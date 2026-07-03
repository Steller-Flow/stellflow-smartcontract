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
fn test_release_success() {
    let (env, contract, client, freelancer, token) = setup();
    let escrow_id = contract.create_escrow(client.clone(), freelancer, token, 1000);
    contract.fund_escrow(client.clone(), escrow_id);
    contract.release(client.clone(), escrow_id);
    let escrow = contract.get_escrow(escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Released);
    assert!(escrow.released_at.is_some());
}

#[test]
fn test_release_not_funded() {
    let (env, contract, client, freelancer, token) = setup();
    let escrow_id = contract.create_escrow(client, freelancer, token, 1000);
    let result = contract.try_release(client, escrow_id);
    assert!(result.is_err());
}

#[test]
fn test_release_wrong_client() {
    let (env, contract, client, freelancer, token) = setup();
    let wrong_client = Address::generate(&env);
    let escrow_id = contract.create_escrow(client, freelancer, token, 1000);
    contract.fund_escrow(client, escrow_id);
    let result = contract.try_release(wrong_client, escrow_id);
    assert!(result.is_err());
}

#[test]
fn test_release_already_released() {
    let (env, contract, client, freelancer, token) = setup();
    let escrow_id = contract.create_escrow(client.clone(), freelancer, token, 1000);
    contract.fund_escrow(client.clone(), escrow_id);
    contract.release(client.clone(), escrow_id);
    let result = contract.try_release(client, escrow_id);
    assert!(result.is_err());
}

#[test]
fn test_release_already_refunded() {
    let (env, contract, client, freelancer, token) = setup();
    let escrow_id = contract.create_escrow(client.clone(), freelancer, token, 1000);
    contract.fund_escrow(client.clone(), escrow_id);
    contract.refund(client.clone(), escrow_id);
    let result = contract.try_release(client, escrow_id);
    assert!(result.is_err());
}

#[test]
fn test_refund_success() {
    let (env, contract, client, freelancer, token) = setup();
    let escrow_id = contract.create_escrow(client.clone(), freelancer, token, 1000);
    contract.fund_escrow(client.clone(), escrow_id);
    contract.refund(client.clone(), escrow_id);
    let escrow = contract.get_escrow(escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Refunded);
    assert!(escrow.refunded_at.is_some());
}

#[test]
fn test_refund_not_funded() {
    let (env, contract, client, freelancer, token) = setup();
    let escrow_id = contract.create_escrow(client, freelancer, token, 1000);
    let result = contract.try_refund(client, escrow_id);
    assert!(result.is_err());
}

#[test]
fn test_refund_wrong_client() {
    let (env, contract, client, freelancer, token) = setup();
    let wrong_client = Address::generate(&env);
    let escrow_id = contract.create_escrow(client, freelancer, token, 1000);
    contract.fund_escrow(client, escrow_id);
    let result = contract.try_refund(wrong_client, escrow_id);
    assert!(result.is_err());
}

#[test]
fn test_refund_already_refunded() {
    let (env, contract, client, freelancer, token) = setup();
    let escrow_id = contract.create_escrow(client.clone(), freelancer, token, 1000);
    contract.fund_escrow(client.clone(), escrow_id);
    contract.refund(client.clone(), escrow_id);
    let result = contract.try_refund(client, escrow_id);
    assert!(result.is_err());
}

#[test]
fn test_refund_already_released() {
    let (env, contract, client, freelancer, token) = setup();
    let escrow_id = contract.create_escrow(client.clone(), freelancer, token, 1000);
    contract.fund_escrow(client.clone(), escrow_id);
    contract.release(client.clone(), escrow_id);
    let result = contract.try_refund(client, escrow_id);
    assert!(result.is_err());
}
