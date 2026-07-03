#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, Env, Vec};
use stellflow_escrow::{EscrowContract, EscrowStatus, MilestoneStatus};

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
fn test_full_lifecycle_create_fund_release() {
    let (env, contract, client, freelancer, token) = setup();
    let escrow_id = contract.create_escrow(
        client.clone(),
        freelancer.clone(),
        token.clone(),
        5000,
    );
    assert_eq!(escrow_id, 1);

    contract.fund_escrow(client.clone(), escrow_id);
    let escrow = contract.get_escrow(escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Funded);

    contract.release(client.clone(), escrow_id);
    let escrow = contract.get_escrow(escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Released);
    assert!(escrow.released_at.is_some());
}

#[test]
fn test_full_lifecycle_create_fund_refund() {
    let (env, contract, client, freelancer, token) = setup();
    let escrow_id = contract.create_escrow(
        client.clone(),
        freelancer.clone(),
        token.clone(),
        3000,
    );

    contract.fund_escrow(client.clone(), escrow_id);
    contract.refund(client.clone(), escrow_id);

    let escrow = contract.get_escrow(escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Refunded);
    assert!(escrow.refunded_at.is_some());
}

#[test]
fn test_full_lifecycle_create_fund_dispute_resolve() {
    let (env, contract, client, freelancer, token) = setup();
    let escrow_id = contract.create_escrow(
        client.clone(),
        freelancer.clone(),
        token.clone(),
        10000,
    );

    contract.fund_escrow(client.clone(), escrow_id);

    contract.raise_dispute(freelancer.clone(), escrow_id);
    let escrow = contract.get_escrow(escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Disputed);

    contract.initialize_admin(client.clone());
    contract.resolve_dispute(client.clone(), escrow_id, true);
    let escrow = contract.get_escrow(escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Released);
}

#[test]
fn test_full_lifecycle_create_cancel() {
    let (env, contract, client, freelancer, token) = setup();
    let escrow_id = contract.create_escrow(
        client.clone(),
        freelancer.clone(),
        token.clone(),
        2000,
    );

    contract.cancel_escrow(client.clone(), escrow_id);
    let escrow = contract.get_escrow(escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Cancelled);
}

#[test]
fn test_milestone_lifecycle() {
    let (env, contract, client, freelancer, token) = setup();
    let mut descs = Vec::new(&env);
    descs.push_back(soroban_sdk::String::from_str(&env, "Design phase"));
    descs.push_back(soroban_sdk::String::from_str(&env, "Development phase"));
    let mut amounts = Vec::new(&env);
    amounts.push_back(5000);
    amounts.push_back(5000);

    let escrow_id = contract.create_escrow_with_milestones(
        client.clone(),
        freelancer.clone(),
        token.clone(),
        10000,
        descs,
        amounts,
        None,
    );

    contract.fund_escrow(client.clone(), escrow_id);

    contract.approve_milestone(client.clone(), escrow_id, 0);
    let escrow = contract.get_escrow(escrow_id);
    assert_eq!(escrow.milestones.get(0).unwrap().status, MilestoneStatus::Approved);

    contract.approve_milestone(client.clone(), escrow_id, 1);
    let escrow = contract.get_escrow(escrow_id);
    assert_eq!(escrow.milestones.get(1).unwrap().status, MilestoneStatus::Approved);
}

#[test]
fn test_history_tracking() {
    let (env, contract, client, freelancer, token) = setup();
    let escrow_id = contract.create_escrow(
        client.clone(),
        freelancer.clone(),
        token.clone(),
        1000,
    );

    contract.fund_escrow(client.clone(), escrow_id);
    contract.release(client.clone(), escrow_id);

    let history = contract.get_history(escrow_id);
    assert!(history.len() >= 2);
}

#[test]
fn test_modify_before_funding() {
    let (env, contract, client, freelancer, token) = setup();
    let escrow_id = contract.create_escrow(
        client.clone(),
        freelancer.clone(),
        token.clone(),
        1000,
    );

    let new_freelancer = Address::generate(&env);
    contract.modify_escrow(
        client.clone(),
        escrow_id,
        Some(new_freelancer.clone()),
        Some(2000),
    );

    let escrow = contract.get_escrow(escrow_id);
    assert_eq!(escrow.freelancer, new_freelancer);
    assert_eq!(escrow.amount, 2000);
}

#[test]
fn test_paused_contract_blocks_operations() {
    let (env, contract, client, freelancer, token) = setup();
    contract.initialize_admin(client.clone());
    contract.set_paused(client.clone(), true);

    let result = contract.try_create_escrow(
        client.clone(),
        freelancer,
        token,
        1000,
    );
    assert!(result.is_err());
}
