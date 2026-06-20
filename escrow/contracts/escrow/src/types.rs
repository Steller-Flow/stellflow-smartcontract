use soroban_sdk::{contracttype, Address};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EscrowStatus {
    Pending,
    Funded,
    Released,
    Refunded,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct Escrow {
    pub escrow_id: u64,
    pub client: Address,
    pub freelancer: Address,
    pub token: Address,
    pub amount: i128,
    pub status: EscrowStatus,
    pub created_at: u64,
    pub funded_at: Option<u64>,
    pub released_at: Option<u64>,
    pub refunded_at: Option<u64>,
}

#[contracttype]
pub enum DataKey {
    Escrow(u64),
    EscrowCounter,
}
