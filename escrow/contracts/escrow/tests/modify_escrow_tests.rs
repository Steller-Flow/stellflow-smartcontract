#![cfg(test)]

use soroban_sdk::{testutils::Address as _, testutils::Events, Address, Env};
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
fn test_modify_escrow_change_amount() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    c.modify_escrow(&client, &escrow_id, &None, &Some(20000));
    let escrow = c.get_escrow(&escrow_id);
    assert_eq!(escrow.amount, 20000);
    assert_eq!(escrow.freelancer, freelancer);
}

#[test]
fn test_modify_escrow_change_freelancer() {
    let (env, client, freelancer, token) = setup();
    let new_freelancer = Address::generate(&env);
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    c.modify_escrow(&client, &escrow_id, &Some(new_freelancer.clone()), &None);
    let escrow = c.get_escrow(&escrow_id);
    assert_eq!(escrow.freelancer, new_freelancer);
    assert_eq!(escrow.amount, 10000);
}

#[test]
fn test_modify_escrow_change_both() {
    let (env, client, freelancer, token) = setup();
    let new_freelancer = Address::generate(&env);
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    c.modify_escrow(&client, &escrow_id, &Some(new_freelancer.clone()), &Some(5000));
    let escrow = c.get_escrow(&escrow_id);
    assert_eq!(escrow.freelancer, new_freelancer);
    assert_eq!(escrow.amount, 5000);
}

#[test]
fn test_modify_escrow_wrong_client() {
    let (env, client, freelancer, token) = setup();
    let wrong_client = Address::generate(&env);
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    let result = c.try_modify_escrow(&wrong_client, &escrow_id, &None, &Some(20000));
    assert!(result.is_err());
}

#[test]
fn test_modify_escrow_not_found() {
    let (env, client, _, _) = setup();
    let c = contract(&env);
    let result = c.try_modify_escrow(&client, &999, &None, &Some(20000));
    assert!(result.is_err());
}

#[test]
fn test_modify_escrow_already_funded() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    c.fund_escrow(&client, &escrow_id);
    let result = c.try_modify_escrow(&client, &escrow_id, &None, &Some(20000));
    assert!(result.is_err());
}

#[test]
fn test_modify_escrow_already_released() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    c.fund_escrow(&client, &escrow_id);
    c.release(&client, &escrow_id);
    let result = c.try_modify_escrow(&client, &escrow_id, &None, &Some(20000));
    assert!(result.is_err());
}

#[test]
fn test_modify_escrow_invalid_amount_zero() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    let result = c.try_modify_escrow(&client, &escrow_id, &None, &Some(0));
    assert!(result.is_err());
}

#[test]
fn test_modify_escrow_invalid_amount_negative() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    let result = c.try_modify_escrow(&client, &escrow_id, &None, &Some(-1000));
    assert!(result.is_err());
}

#[test]
fn test_modify_escrow_same_freelancer_as_client() {
    let (env, client, _, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &Address::generate(&env), &token, &10000, &None);
    let result = c.try_modify_escrow(&client, &escrow_id, &Some(client.clone()), &None);
    assert!(result.is_err());
}

#[test]
fn test_modify_escrow_no_change() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    c.modify_escrow(&client, &escrow_id, &None, &None);
    let escrow = c.get_escrow(&escrow_id);
    assert_eq!(escrow.amount, 10000);
    assert_eq!(escrow.freelancer, freelancer);
}

#[test]
fn test_modify_escrow_emits_event() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    c.modify_escrow(&client, &escrow_id, &None, &Some(20000));
    let escrow = c.get_escrow(&escrow_id);
    assert_eq!(escrow.amount, 20000);
}

#[test]
fn test_modify_escrow_cannot_change_after_funding() {
    let (env, client, freelancer, token) = setup();
    let c = contract(&env);
    let escrow_id = c.create_escrow(&client, &freelancer, &token, &10000, &None);
    c.fund_escrow(&client, &escrow_id);
    let result = c.try_modify_escrow(&client, &escrow_id, &None, &None);
    assert!(result.is_err());
}
