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

    let contract_id = env.register_contract(None, EscrowContract);
    let c = crate::contract::EscrowContractClient::new(env, &contract_id);
    let escrow_id = c.create_escrow(client, freelancer, token, &amount, &None);
    c.fund_escrow(client, &escrow_id);
    escrow_id
}
