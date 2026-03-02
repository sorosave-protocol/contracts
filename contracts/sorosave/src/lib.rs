#![no_std]

use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, String, Vec};

mod admin;
mod contribution;
mod errors;
mod group;
mod payout;
mod storage;
mod types;

pub use errors::ContractError;
pub use types::*;

#[contract]
pub struct SoroSaveContract;

#[contractimpl]
impl SoroSaveContract {
    /// Initialize the protocol with a global admin.
    pub fn __constructor(env: Env, admin: Address) {
        if storage::has_admin(&env) {
            panic!("already initialized");
        }
        storage::set_admin(&env, &admin);
    }

    // ─── Group Lifecycle ────────────────────────────────────────────

    /// Create a new savings group. The caller becomes the group admin and first member.
    pub fn create_group(
        env: Env,
        admin: Address,
        name: String,
        token: Address,
        contribution_amount: i128,
        cycle_length: u64,
        max_members: u32,
    ) -> Result<u64, ContractError> {
        group::create_group(
            &env,
            admin,
            name,
            token,
            contribution_amount,
            cycle_length,
            max_members,
        )
    }

    /// Join an existing group that is still forming.
    pub fn join_group(env: Env, member: Address, group_id: u64) -> Result<(), ContractError> {
        group::join_group(&env, member, group_id)
    }

    /// Leave a group (only allowed while group is still forming).
    pub fn leave_group(env: Env, member: Address, group_id: u64) -> Result<(), ContractError> {
        group::leave_group(&env, member, group_id)
    }

    /// Start the group rounds. Only the group admin can call this.
    pub fn start_group(env: Env, admin: Address, group_id: u64) -> Result<(), ContractError> {
        group::start_group(&env, admin, group_id)
    }

    /// Get group details.
    pub fn get_group(env: Env, group_id: u64) -> Result<SavingsGroup, ContractError> {
        group::get_group(&env, group_id)
    }

    /// Get all group IDs a member belongs to.
    pub fn get_member_groups(env: Env, member: Address) -> Vec<u64> {
        group::get_member_groups(&env, member)
    }

    // ─── Group Templates ────────────────────────────────────────────

    /// Save a group template for quick creation later.
    /// Max 10 templates per admin.
    pub fn save_template(
        env: Env,
        admin: Address,
        name: String,
        token: Address,
        contribution_amount: i128,
        cycle_length: u64,
        max_members: u32,
    ) -> Result<u32, ContractError> {
        group::save_template(
            &env,
            admin,
            name,
            token,
            contribution_amount,
            cycle_length,
            max_members,
        )
    }

    /// Get a saved template by ID.
    pub fn get_template(
        env: Env,
        admin: Address,
        template_id: u32,
    ) -> Result<GroupTemplate, ContractError> {
        group::get_template(&env, admin, template_id)
    }

    /// Create a new group from a saved template.
    pub fn create_from_template(
        env: Env,
        admin: Address,
        template_id: u32,
        name: String,
    ) -> Result<u64, ContractError> {
        group::create_from_template(&env, admin, template_id, name)
    }

    // ─── Contributions ──────────────────────────────────────────────

    /// Contribute to the current round of a group.
    pub fn contribute(env: Env, member: Address, group_id: u64) -> Result<(), ContractError> {
        contribution::contribute(&env, member, group_id)
    }

    /// Get the status of a specific round.
    pub fn get_round_status(
        env: Env,
        group_id: u64,
        round: u32,
    ) -> Result<RoundInfo, ContractError> {
        contribution::get_round_status(&env, group_id, round)
    }

    /// Check if a member has contributed in a specific round.
    pub fn has_contributed(
        env: Env,
        member: Address,
        group_id: u64,
        round: u32,
    ) -> Result<bool, ContractError> {
        contribution::has_contributed(&env, member, group_id, round)
    }

    // ─── Payouts ────────────────────────────────────────────────────

    /// Distribute the pot to the current round's recipient. Anyone can call this
    /// once all contributions are in.
    pub fn distribute_payout(env: Env, group_id: u64) -> Result<(), ContractError> {
        payout::distribute_payout(&env, group_id)
    }

    /// Get the payout order for a group.
    pub fn get_payout_order(env: Env, group_id: u64) -> Result<Vec<Address>, ContractError> {
        payout::get_payout_order(&env, group_id)
    }

    /// Get the current round's recipient.
    pub fn get_current_recipient(env: Env, group_id: u64) -> Result<Address, ContractError> {
        payout::get_current_recipient(&env, group_id)
    }

    // ─── Admin / Governance ─────────────────────────────────────────

    /// Pause an active group.
    pub fn pause_group(env: Env, admin: Address, group_id: u64) -> Result<(), ContractError> {
        admin::pause_group(&env, admin, group_id)
    }

    /// Resume a paused group.
    pub fn resume_group(env: Env, admin: Address, group_id: u64) -> Result<(), ContractError> {
        admin::resume_group(&env, admin, group_id)
    }

    /// Raise a dispute on a group.
    pub fn raise_dispute(
        env: Env,
        member: Address,
        group_id: u64,
        reason: String,
    ) -> Result<(), ContractError> {
        admin::raise_dispute(&env, member, group_id, reason)
    }

    /// Resolve a dispute (group admin or protocol admin).
    pub fn resolve_dispute(env: Env, admin: Address, group_id: u64) -> Result<(), ContractError> {
        admin::resolve_dispute(&env, admin, group_id)
    }

    /// Emergency withdraw — distribute remaining funds proportionally to all members.
    pub fn emergency_withdraw(
        env: Env,
        admin: Address,
        group_id: u64,
    ) -> Result<(), ContractError> {
        admin::emergency_withdraw(&env, admin, group_id)
    }

    /// Transfer group admin role.
    pub fn set_group_admin(
        env: Env,
        current_admin: Address,
        group_id: u64,
        new_admin: Address,
    ) -> Result<(), ContractError> {
        admin::set_group_admin(&env, current_admin, group_id, new_admin)
    }
}

#[cfg(test)]
mod test;
