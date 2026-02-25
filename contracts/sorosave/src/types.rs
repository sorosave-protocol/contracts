use soroban_sdk::{contracttype, Address, Map, String, Vec};

/// Status of a savings group throughout its lifecycle.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum GroupStatus {
    Forming,   // Accepting members, not yet started
    Active,    // Rounds in progress
    Completed, // All rounds finished, all payouts distributed
    Disputed,  // A dispute has been raised, group is frozen
    Paused,    // Admin has paused the group
}

/// Core savings group configuration and state.
#[contracttype]
#[derive(Clone, Debug)]
pub struct SavingsGroup {
    pub id: u64,
    pub name: String,
    pub admin: Address,
    pub token: Address,
    pub contribution_amount: i128,
    pub deposit_amount: i128,
    pub cycle_length: u64,
    pub max_members: u32,
    pub members: Vec<Address>,
    pub payout_order: Vec<Address>,
    pub current_round: u32,
    pub total_rounds: u32,
    pub status: GroupStatus,
    pub created_at: u64,
}

/// Tracks contributions and payout status for a single round.
#[contracttype]
#[derive(Clone, Debug)]
pub struct RoundInfo {
    pub round_number: u32,
    pub recipient: Address,
    pub contributions: Map<Address, bool>,
    pub total_contributed: i128,
    pub is_complete: bool,
    pub deadline: u64,
}

/// Dispute information for a group.
#[contracttype]
#[derive(Clone, Debug)]
pub struct Dispute {
    pub raised_by: Address,
    pub reason: String,
    pub raised_at: u64,
}

/// Storage keys for all contract data.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    GroupCounter,
    Group(u64),
    Round(u64, u32),
    MemberGroups(Address),
    Dispute(u64),
    Deposits(u64),
}
