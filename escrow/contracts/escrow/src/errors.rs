use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum EscrowError {
    /// amount must be > 0
    InvalidAmount = 1,
    /// caller is not the authorised party
    Unauthorized = 2,
    /// no escrow found for the given id
    EscrowNotFound = 3,
    /// escrow has already been funded
    AlreadyFunded = 4,
    /// escrow is in the wrong state for this operation
    InvalidStatus = 5,
    /// escrow has already been released
    AlreadyReleased = 6,
    /// escrow has already been refunded
    AlreadyRefunded = 7,
    /// token transfer failed
    TransferFailed = 8,
}
