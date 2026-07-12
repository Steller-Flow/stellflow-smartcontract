#![allow(deprecated)]
#![cfg(test)]

use soroban_sdk::{testutils::Address as _, testutils::Ledger, Address, Env, String, Vec};
use stellflow_escrow::{EscrowContract, EscrowStatus};

fn setup() -> (Env, Address, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| li.timestamp = 1000);
    let client = Address::generate(&env);
    let freelancer = Address::generate(&env);
    let admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
    let token = token_contract.address();
    let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token);
    token_client.mint(&client, &100_000_000);
    (env, client, freelancer, admin, token)
}

fn contract(env: &Env) -> stellflow_escrow::contract::EscrowContractClient<'_> {
    let contract_id = env.register_contract(None, EscrowContract);
    stellflow_escrow::contract::EscrowContractClient::new(env, &contract_id)
}

fn token_balance(env: &Env, token: &Address, account: &Address) -> i128 {
    let client = soroban_sdk::token::Client::new(env, token);
    client.balance(account)
}

#[test]
fn test_full_lifecycle_with_balance_verification() {
    let (env, client, freelancer, _admin, token) = setup();
    let c = contract(&env);

    let client_balance_before = token_balance(&env, &token, &client);
    let freelancer_balance_before = token_balance(&env, &token, &freelancer);

    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    let escrow = c.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Pending);

    c.fund_escrow(&client, &escrow_id);
    let client_balance_after_fund = token_balance(&env, &token, &client);
    assert_eq!(client_balance_before - client_balance_after_fund, 10000);

    c.release(&client, &escrow_id);
    let client_balance_after_release = token_balance(&env, &token, &client);
    let freelancer_balance_after_release = token_balance(&env, &token, &freelancer);
    assert_eq!(
        freelancer_balance_after_release - freelancer_balance_before,
        10000
    );
    assert_eq!(client_balance_after_release, client_balance_after_fund);

    let escrow = c.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Released);
    assert_eq!(escrow.total_released, 10000);
}

#[test]
fn test_refund_with_balance_verification() {
    let (env, client, freelancer, _admin, token) = setup();
    let c = contract(&env);

    let client_balance_before = token_balance(&env, &token, &client);

    let escrow_id = c.create_escrow(&client, &freelancer, &token, &5000, &None);
    c.fund_escrow(&client, &escrow_id);
    c.refund(&client, &escrow_id);

    let client_balance_after = token_balance(&env, &token, &client);
    assert_eq!(client_balance_before, client_balance_after);

    let escrow = c.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Refunded);
    assert_eq!(escrow.total_refunded, 5000);
}

#[test]
fn test_fee_mechanism_on_release() {
    let (env, client, freelancer, admin, token) = setup();
    let c = contract(&env);

    c.initialize_admin(&admin);
    c.set_treasury(&admin, &admin);
    c.set_default_fee(&admin, &2);

    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    c.fund_escrow(&client, &escrow_id);

    let freelancer_balance_before = token_balance(&env, &token, &freelancer);
    let treasury_balance_before = token_balance(&env, &token, &admin);

    c.release(&client, &escrow_id);

    let freelancer_balance_after = token_balance(&env, &token, &freelancer);
    let treasury_balance_after = token_balance(&env, &token, &admin);

    let expected_fee = 10000 * 2 / 100;
    let expected_release = 10000 - expected_fee;

    assert_eq!(
        freelancer_balance_after - freelancer_balance_before,
        expected_release
    );
    assert_eq!(
        treasury_balance_after - treasury_balance_before,
        expected_fee
    );

    let escrow = c.get_escrow(&escrow_id);
    assert_eq!(escrow.total_released, expected_release);
}

#[test]
fn test_fee_mechanism_on_dispute_resolution() {
    let (env, client, freelancer, admin, token) = setup();
    let c = contract(&env);

    c.initialize_admin(&admin);
    c.set_treasury(&admin, &admin);
    c.set_default_fee(&admin, &5);

    let escrow_id = c.create_escrow(&client, &freelancer, &token, &20000, &None);
    c.fund_escrow(&client, &escrow_id);
    c.raise_dispute(&freelancer, &escrow_id);

    let freelancer_balance_before = token_balance(&env, &token, &freelancer);
    let treasury_balance_before = token_balance(&env, &token, &admin);

    c.resolve_dispute(&admin, &escrow_id, &true, &None);

    let freelancer_balance_after = token_balance(&env, &token, &freelancer);
    let treasury_balance_after = token_balance(&env, &token, &admin);

    let expected_fee = 20000 * 5 / 100;
    let expected_release = 20000 - expected_fee;

    assert_eq!(
        freelancer_balance_after - freelancer_balance_before,
        expected_release
    );
    assert_eq!(
        treasury_balance_after - treasury_balance_before,
        expected_fee
    );

    let escrow = c.get_escrow(&escrow_id);
    assert_eq!(escrow.total_released, expected_release);
}

#[test]
fn test_no_fee_when_zero_percent() {
    let (env, client, freelancer, _admin, token) = setup();
    let c = contract(&env);

    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    c.fund_escrow(&client, &escrow_id);

    let freelancer_balance_before = token_balance(&env, &token, &freelancer);
    c.release(&client, &escrow_id);
    let freelancer_balance_after = token_balance(&env, &token, &freelancer);

    assert_eq!(freelancer_balance_after - freelancer_balance_before, 10000);

    let escrow = c.get_escrow(&escrow_id);
    assert_eq!(escrow.total_released, 10000);
}

#[test]
fn test_history_after_create_fund_release() {
    let (env, client, freelancer, _admin, token) = setup();
    let c = contract(&env);

    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    c.fund_escrow(&client, &escrow_id);
    c.release(&client, &escrow_id);

    let history = c.get_history(&escrow_id);
    assert_eq!(history.len(), 2);

    let fund_event = history.get(0).unwrap();
    assert_eq!(fund_event.from_status, EscrowStatus::Pending);
    assert_eq!(fund_event.to_status, EscrowStatus::Funded);
    assert_eq!(fund_event.actor, client);
    assert_eq!(fund_event.amount, 10000);

    let release_event = history.get(1).unwrap();
    assert_eq!(release_event.from_status, EscrowStatus::Funded);
    assert_eq!(release_event.to_status, EscrowStatus::Released);
    assert_eq!(release_event.actor, client);
    assert_eq!(release_event.amount, 10000);
}

#[test]
fn test_history_after_cancel() {
    let (env, client, freelancer, _admin, token) = setup();
    let c = contract(&env);

    let escrow_id = c.create_escrow(&client, &freelancer, &token, &5000, &None);
    c.cancel_escrow(&client, &escrow_id);

    let history = c.get_history(&escrow_id);
    assert_eq!(history.len(), 1);

    let cancel_event = history.get(0).unwrap();
    assert_eq!(cancel_event.from_status, EscrowStatus::Pending);
    assert_eq!(cancel_event.to_status, EscrowStatus::Cancelled);
    assert_eq!(cancel_event.actor, client);
    assert_eq!(cancel_event.amount, 0);
}

#[test]
fn test_history_after_dispute_resolution() {
    let (env, client, freelancer, admin, token) = setup();
    let c = contract(&env);

    c.initialize_admin(&admin);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &15000, &None);
    c.fund_escrow(&client, &escrow_id);
    c.raise_dispute(&freelancer, &escrow_id);
    c.resolve_dispute(&admin, &escrow_id, &true, &None);

    let history = c.get_history(&escrow_id);
    assert_eq!(history.len(), 3);

    let fund_event = history.get(0).unwrap();
    assert_eq!(fund_event.from_status, EscrowStatus::Pending);
    assert_eq!(fund_event.to_status, EscrowStatus::Funded);

    let dispute_event = history.get(1).unwrap();
    assert_eq!(dispute_event.from_status, EscrowStatus::Funded);
    assert_eq!(dispute_event.to_status, EscrowStatus::Disputed);
    assert_eq!(dispute_event.actor, freelancer);

    let resolve_event = history.get(2).unwrap();
    assert_eq!(resolve_event.from_status, EscrowStatus::Disputed);
    assert_eq!(resolve_event.to_status, EscrowStatus::Released);
    assert_eq!(resolve_event.actor, admin);
}

#[test]
fn test_milestone_full_lifecycle_with_balances() {
    let (env, client, freelancer, _admin, token) = setup();
    let c = contract(&env);

    let mut descs = Vec::new(&env);
    descs.push_back(String::from_str(&env, "Design"));
    descs.push_back(String::from_str(&env, "Development"));
    let mut amounts = Vec::new(&env);
    amounts.push_back(4000);
    amounts.push_back(6000);

    let escrow_id = c.create_escrow_with_milestones(
        &client,
        &freelancer,
        &token,
        &10000,
        &descs,
        &amounts,
        &None,
    );

    c.fund_escrow(&client, &escrow_id);

    let freelancer_balance_before = token_balance(&env, &token, &freelancer);

    c.approve_milestone(&client, &escrow_id, &0);
    c.release_milestone(&client, &escrow_id, &0);

    let freelancer_balance_after_milestone1 = token_balance(&env, &token, &freelancer);
    assert_eq!(
        freelancer_balance_after_milestone1 - freelancer_balance_before,
        4000
    );

    c.approve_milestone(&client, &escrow_id, &1);
    c.release_milestone(&client, &escrow_id, &1);

    let freelancer_balance_after_milestone2 = token_balance(&env, &token, &freelancer);
    assert_eq!(
        freelancer_balance_after_milestone2 - freelancer_balance_before,
        10000
    );

    let escrow = c.get_escrow(&escrow_id);
    assert_eq!(escrow.total_released, 10000);
    assert!(escrow.milestones.get(0).unwrap().released);
    assert!(escrow.milestones.get(1).unwrap().released);
}

#[test]
fn test_timeout_claim_with_balance_verification() {
    let (env, client, freelancer, _admin, token) = setup();
    let c = contract(&env);

    let client_balance_before = token_balance(&env, &token, &client);

    let escrow_id = c.create_escrow(&client, &freelancer, &token, &8000, &Some(1001));
    c.fund_escrow(&client, &escrow_id);

    env.ledger().with_mut(|li| li.timestamp = 1002);
    c.claim_timeout(&client, &escrow_id);

    let client_balance_after = token_balance(&env, &token, &client);
    assert_eq!(client_balance_before, client_balance_after);

    let escrow = c.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Refunded);
    assert_eq!(escrow.total_refunded, 8000);
}

#[test]
fn test_multiple_escrows_independent_balances() {
    let (env, client, freelancer, _admin, token) = setup();
    let c = contract(&env);

    let escrow1 = c.create_escrow(&client, &freelancer, &token, &3000, &None);
    let escrow2 = c.create_escrow(&client, &freelancer, &token, &7000, &None);

    c.fund_escrow(&client, &escrow1);
    c.fund_escrow(&client, &escrow2);

    c.release(&client, &escrow1);
    c.refund(&client, &escrow2);

    let e1 = c.get_escrow(&escrow1);
    let e2 = c.get_escrow(&escrow2);
    assert_eq!(e1.status, EscrowStatus::Released);
    assert_eq!(e2.status, EscrowStatus::Refunded);
    assert_eq!(e1.total_released, 3000);
    assert_eq!(e2.total_refunded, 7000);
}

#[test]
fn test_history_entries_have_timestamps() {
    let (env, client, freelancer, _admin, token) = setup();
    let c = contract(&env);

    let escrow_id = c.create_escrow(&client, &freelancer, &token, &1000, &None);
    c.fund_escrow(&client, &escrow_id);
    c.release(&client, &escrow_id);

    let history = c.get_history(&escrow_id);
    for i in 0..history.len() {
        let event = history.get(i).unwrap();
        assert!(event.timestamp > 0);
    }
}

#[test]
fn test_set_fee_and_release_with_new_fee() {
    let (env, client, freelancer, admin, token) = setup();
    let c = contract(&env);

    c.initialize_admin(&admin);
    c.set_treasury(&admin, &admin);

    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    c.set_default_fee(&admin, &0);
    c.fund_escrow(&client, &escrow_id);
    c.set_fee(&admin, &escrow_id, &3);

    let freelancer_balance_before = token_balance(&env, &token, &freelancer);
    c.release(&client, &escrow_id);
    let freelancer_balance_after = token_balance(&env, &token, &freelancer);

    let expected_fee = 10000 * 3 / 100;
    assert_eq!(
        freelancer_balance_after - freelancer_balance_before,
        10000 - expected_fee
    );
}
