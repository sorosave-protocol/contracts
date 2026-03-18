use soroban_sdk::{contracttype, Address, Map, Vec};

#[derive(Clone)]
#[contracttype]
pub struct GroupInfo {
    pub name: String,
    pub description: String,
    pub admin: Address,
    pub members: Vec<Address>,
    pub target_amount: i128,
    pub contribution_amount: i128,
    pub frequency: u32, // in seconds
    pub is_active: bool,
    pub created_at: u64,
    pub referral_count: u32,
}

#[derive(Clone)]
#[contracttype]
pub struct MemberInfo {
    pub address: Address,
    pub total_contributed: i128,
    pub last_contribution: u64,
    pub is_active: bool,
    pub joined_at: u64,
    pub referrer: Option<Address>,
    pub referral_count: u32,
}

#[derive(Clone)]
#[contracttype]
pub struct ContributionRecord {
    pub member: Address,
    pub amount: i128,
    pub timestamp: u64,
    pub group_id: u32,
}

#[derive(Clone)]
#[contracttype]
pub struct ReferralReward {
    pub referrer: Address,
    pub referred: Address,
    pub reward_amount: i128,
    pub timestamp: u64,
}

#[contracttype]
pub enum DataKey {
    GroupCounter,
    GroupInfo(u32),
    MemberInfo(u32, Address),
    ContributionHistory(u32),
    ReferralRewards(Address),
    GroupReferrals(u32),
    MemberReferrals(Address),
}