use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    Unauthorized = 3,
    GroupNotFound = 4,
    GroupFull = 5,
    AlreadyMember = 6,
    NotMember = 7,
    GroupNotActive = 8,
    AlreadyContributed = 9,
    RoundNotActive = 10,
    InvalidAmount = 11,
    GroupNotForming = 12,
    PayoutFailed = 13,
    GroupPaused = 14,
    DisputeActive = 15,
    InsufficientMembers = 16,
    RoundNotComplete = 17,
    GroupCompleted = 18,
}
