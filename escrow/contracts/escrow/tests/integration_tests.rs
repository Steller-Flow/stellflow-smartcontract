#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};
use stellflow_escrow::{EscrowContract, EscrowStatus, MilestoneStatus};

fn setup() -> (Env, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let client = Address::generate(&env);
    let freelancer = Address::generate(&env);
    let admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(admin);
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
fn test_full_lifecycle_create_fund_release() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &5000, &None);
    assert_eq!(escrow_id, 1);
    c.fund_escrow(&client, &escrow_id);
    c.release(&client, &escrow_id);
    let escrow = c.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Released);
    assert!(escrow.released_at.is_some());
}

#[test]
fn test_full_lifecycle_create_fund_refund() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &3000, &None);
    c.fund_escrow(&client, &escrow_id);
    c.refund(&client, &escrow_id);
    let escrow = c.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Refunded);
}

#[test]
fn test_full_lifecycle_create_fund_dispute_resolve() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    c.fund_escrow(&client, &escrow_id);
    c.raise_dispute(&freelancer, &escrow_id);
    let escrow = c.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Disputed);
    c.initialize_admin(&client);
    c.resolve_dispute(&client, &escrow_id, &true, &None);
    let escrow = c.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Released);
}

#[test]
fn test_full_lifecycle_create_cancel() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &2000, &None);
    c.cancel_escrow(&client, &escrow_id);
    let escrow = c.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Cancelled);
}

#[test]
fn test_milestone_lifecycle() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let mut descs = Vec::new(&env);
    descs.push_back(String::from_str(&env, "Design"));
    descs.push_back(String::from_str(&env, "Dev"));
    let mut amounts = Vec::new(&env);
    amounts.push_back(5000);
    amounts.push_back(5000);
    let escrow_id = c.create_escrow_with_milestones(&client, &freelancer, &token, &10000, &descs, &amounts, &None);
    c.fund_escrow(&client, &escrow_id);
    c.approve_milestone(&client, &escrow_id, &0);
    let escrow = c.get_escrow(&escrow_id);
    assert_eq!(escrow.milestones.get(0).unwrap().status, MilestoneStatus::Approved);
    c.approve_milestone(&client, &escrow_id, &1);
    let escrow = c.get_escrow(&escrow_id);
    assert_eq!(escrow.milestones.get(1).unwrap().status, MilestoneStatus::Approved);
}

#[test]
fn test_history_tracking() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &1000, &None);
    c.fund_escrow(&client, &escrow_id);
    c.release(&client, &escrow_id);
    let history = c.get_history(&escrow_id);
    assert!(history.len() >= 2);
}

#[test]
fn test_modify_before_funding() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &1000, &None);
    let new_freelancer = Address::generate(&env);
    c.modify_escrow(&client, &escrow_id, &Some(new_freelancer.clone()), &Some(2000));
    let escrow = c.get_escrow(&escrow_id);
    assert_eq!(escrow.freelancer, new_freelancer);
    assert_eq!(escrow.amount, 2000);
}

#[test]
fn test_paused_contract_blocks_operations() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    c.initialize_admin(&client);
    c.set_paused(&client, &true);
    let result = c.try_create_escrow(&client, &freelancer, &token, &1000, &None);
    assert!(result.is_err());
}

#[test]
fn test_milestone_amount_mismatch() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let mut descs = Vec::new(&env);
    descs.push_back(String::from_str(&env, "Phase 1"));
    descs.push_back(String::from_str(&env, "Phase 2"));
    let mut amounts = Vec::new(&env);
    amounts.push_back(5000);
    amounts.push_back(5000);
    let result = c.try_create_escrow_with_milestones(&client, &freelancer, &token, &11000, &descs, &amounts, &None);
    assert!(result.is_err());
}

#[test]
fn test_milestone_description_amount_count_mismatch() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let mut descs = Vec::new(&env);
    descs.push_back(String::from_str(&env, "Phase 1"));
    let mut amounts = Vec::new(&env);
    amounts.push_back(5000);
    amounts.push_back(5000);
    let result = c.try_create_escrow_with_milestones(&client, &freelancer, &token, &10000, &descs, &amounts, &None);
    assert!(result.is_err());
}
