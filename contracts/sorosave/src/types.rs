use soroban_sdk::{contracttype, Address, Map};

#[derive(Clone)]
#[contracttype]
pub struct User {
    pub address: Address,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub created_at: u64,
    pub is_active: bool,
}

#[derive(Clone)]
#[contracttype]
pub struct SavingsGroup {
    pub id: u64,
    pub name: String,
    pub description: String,
    pub target_amount: i128,
    pub current_amount: i128,
    pub contribution_amount: i128,
    pub frequency: u64,
    pub start_date: u64,
    pub end_date: u64,
    pub is_active: bool,
    pub creator: Address,
    pub admins: Vec<Address>,
    pub admin_threshold: u32,
    pub members: Vec<Address>,
    pub created_at: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct AdminProposal {
    pub id: u64,
    pub group_id: u64,
    pub proposal_type: AdminProposalType,
    pub proposer: Address,
    pub target: Option<Address>,
    pub value: Option<i128>,
    pub description: String,
    pub approvals: Vec<Address>,
    pub executed: bool,
    pub created_at: u64,
    pub expires_at: u64,
}

#[derive(Clone)]
#[contracttype]
pub enum AdminProposalType {
    AddAdmin,
    RemoveAdmin,
    UpdateThreshold,
    UpdateGroupSettings,
    WithdrawFunds,
    PauseGroup,
    UnpauseGroup,
}

#[derive(Clone)]
#[contracttype]
pub struct Contribution {
    pub id: u64,
    pub group_id: u64,
    pub user: Address,
    pub amount: i128,
    pub timestamp: u64,
    pub is_penalty: bool,
}

#[derive(Clone)]
#[contracttype]
pub struct Withdrawal {
    pub id: u64,
    pub group_id: u64,
    pub user: Address,
    pub amount: i128,
    pub timestamp: u64,
    pub reason: String,
}

#[derive(Clone)]
#[contracttype]
pub struct GroupMembership {
    pub group_id: u64,
    pub user: Address,
    pub joined_at: u64,
    pub is_active: bool,
    pub total_contributed: i128,
    pub missed_contributions: u32,
}

#[derive(Clone)]
#[contracttype]
pub enum SavingsError {
    UserNotFound = 1,
    GroupNotFound = 2,
    InsufficientFunds = 3,
    UnauthorizedAccess = 4,
    GroupInactive = 5,
    AlreadyMember = 6,
    NotMember = 7,
    InvalidAmount = 8,
    GroupFull = 9,
    ContributionPeriodEnded = 10,
    InsufficientApprovals = 11,
    ProposalNotFound = 12,
    ProposalExpired = 13,
    ProposalAlreadyExecuted = 14,
    NotAdmin = 15,
    InvalidThreshold = 16,
}