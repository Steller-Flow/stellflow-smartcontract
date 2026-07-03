#[cfg(test)]
use soroban_sdk::{testutils::Address as _, Address, Env};

#[cfg(test)]
pub fn create_test_env() -> (Env, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let client = Address::generate(&env);
    let freelancer = Address::generate(&env);
    let token = Address::generate(&env);
    (env, client, freelancer, token)
}

#[cfg(test)]
pub fn create_funded_escrow(
    env: &Env,
    client: &Address,
    freelancer: &Address,
    token: &Address,
    amount: i128,
) -> u64 {
    use crate::EscrowContract;

    let contract = EscrowContract::new(env);
    let escrow_id = contract.create_escrow(client.clone(), freelancer.clone(), token.clone(), amount);
    contract.fund_escrow(client.clone(), escrow_id);
    escrow_id
}
