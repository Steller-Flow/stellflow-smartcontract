#![allow(deprecated)]
#![cfg(test)]

use soroban_sdk::{testutils::Address as _, testutils::Ledger, Address, Env};
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
fn test_create_escrow_with_deadline() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &Some(200));
    let escrow = c.get_escrow(&escrow_id);
    assert_eq!(escrow.deadline, Some(200));
}

#[test]
fn test_create_escrow_deadline_in_past() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let result = c.try_create_escrow(&client, &freelancer, &token, &10000, &Some(0));
    assert!(result.is_err());
}

#[test]
fn test_set_deadline_on_pending_escrow() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    c.set_deadline(&client, &escrow_id, &200);
    let escrow = c.get_escrow(&escrow_id);
    assert_eq!(escrow.deadline, Some(200));
}

#[test]
fn test_set_deadline_on_funded_escrow() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    c.fund_escrow(&client, &escrow_id);
    c.set_deadline(&client, &escrow_id, &200);
    let escrow = c.get_escrow(&escrow_id);
    assert_eq!(escrow.deadline, Some(200));
}

#[test]
fn test_set_deadline_wrong_client() {
    let (env, client, freelancer, token) = setup();
    let wrong_client = Address::generate(&env);
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    let result = c.try_set_deadline(&wrong_client, &escrow_id, &200);
    assert!(result.is_err());
}

#[test]
fn test_claim_timeout_success() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &Some(1));
    c.fund_escrow(&client, &escrow_id);
    env.ledger().with_mut(|li| li.timestamp = 2);
    c.claim_timeout(&client, &escrow_id);
    let escrow = c.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Refunded);
    assert_eq!(escrow.total_refunded, 10000);
    assert!(escrow.refunded_at.is_some());
}

#[test]
fn test_claim_timeout_deadline_not_passed() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &Some(100));
    c.fund_escrow(&client, &escrow_id);
    let result = c.try_claim_timeout(&client, &escrow_id);
    assert!(result.is_err());
}

#[test]
fn test_claim_timeout_no_deadline() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    c.fund_escrow(&client, &escrow_id);
    let result = c.try_claim_timeout(&client, &escrow_id);
    assert!(result.is_err());
}

#[test]
fn test_claim_timeout_not_funded() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &Some(1));
    let result = c.try_claim_timeout(&client, &escrow_id);
    assert!(result.is_err());
}

#[test]
fn test_claim_timeout_wrong_client() {
    let (env, client, freelancer, token) = setup();
    let wrong_client = Address::generate(&env);
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &Some(1));
    c.fund_escrow(&client, &escrow_id);
    env.ledger().with_mut(|li| li.timestamp = 2);
    let result = c.try_claim_timeout(&wrong_client, &escrow_id);
    assert!(result.is_err());
}

#[test]
fn test_claim_timeout_already_released() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    c.fund_escrow(&client, &escrow_id);
    c.release(&client, &escrow_id);
    env.ledger().with_mut(|li| li.timestamp = 2);
    let result = c.try_claim_timeout(&client, &escrow_id);
    assert!(result.is_err());
}

#[test]
fn test_claim_timeout_already_refunded() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &Some(1));
    c.fund_escrow(&client, &escrow_id);
    c.refund(&client, &escrow_id);
    env.ledger().with_mut(|li| li.timestamp = 2);
    let result = c.try_claim_timeout(&client, &escrow_id);
    assert!(result.is_err());
}

#[test]
fn test_claim_timeout_tracks_history() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &Some(1));
    c.fund_escrow(&client, &escrow_id);
    env.ledger().with_mut(|li| li.timestamp = 2);
    c.claim_timeout(&client, &escrow_id);
    let history = c.get_history(&escrow_id);
    assert!(history.len() >= 2);
}
