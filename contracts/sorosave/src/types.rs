use soroban_sdk::{contracttype, Address, String, Vec};

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Member {
    pub address: Address,
    pub joined_at: u64,
    pub contribution_amount: i128,
    pub is_active: bool,
    pub referral_count: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Group {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub target_amount: i128,
    pub contribution_frequency: u64,
    pub max_members: u32,
    pub created_at: u64,
    pub is_active: bool,
    pub creator: Address,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Contribution {
    pub member: Address,
    pub group_id: u32,
    pub amount: i128,
    pub timestamp: u64,
    pub round: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Payout {
    pub recipient: Address,
    pub group_id: u32,
    pub amount: i128,
    pub round: u32,
    pub timestamp: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct ReferralEvent {
    pub referrer: Address,
    pub referee: Address,
    pub group_id: u32,
    pub timestamp: u64,
    pub reward_amount: i128,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum DataKey {
    Groups,
    GroupMembers(u32),
    MemberGroups(Address),
    Contributions(u32),
    Payouts(u32),
    NextGroupId,
    GroupCount,
    Referrals,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum GroupStatus {
    Active,
    Completed,
    Cancelled,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct GroupStats {
    pub total_contributed: i128,
    pub current_round: u32,
    pub members_count: u32,
    pub next_payout_round: u32,
}