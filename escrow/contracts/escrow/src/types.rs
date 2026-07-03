use soroban_sdk::{contracttype, Address};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EscrowStatus {
    Pending,
    Funded,
    Released,
    Refunded,
    Cancelled,
    Disputed,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct Milestone {
    pub milestone_id: u32,
    pub description: soroban_sdk::String,
    pub amount: i128,
    pub status: MilestoneStatus,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MilestoneStatus {
    Pending,
    Submitted,
    Approved,
    Rejected,
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
    pub cancelled_at: Option<u64>,
    pub disputed_at: Option<u64>,
    pub deadline: Option<u64>,
    pub milestones: soroban_sdk::Vec<Milestone>,
    pub arbiter: Option<Address>,
    pub fee_percent: u32,
    pub total_released: i128,
    pub total_refunded: i128,
    pub history: soroban_sdk::Vec<EscrowEvent>,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct EscrowEvent {
    pub from_status: EscrowStatus,
    pub to_status: EscrowStatus,
    pub actor: Address,
    pub timestamp: u64,
    pub amount: i128,
}

#[contracttype]
pub enum DataKey {
    Escrow(u64),
    EscrowCounter,
    Admin,
    Paused,
    PlatformTreasury,
    DefaultFeePercent,
}
