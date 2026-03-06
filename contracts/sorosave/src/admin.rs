use soroban_sdk::{Address, Env, String};

use crate::errors::ContractError;
use crate::storage;
use crate::types::{Dispute, DisputeVotes, GroupStatus};

pub fn pause_group(env: &Env, admin: Address, group_id: u64) -> Result<(), ContractError> {
    admin.require_auth();

    let mut group = storage::get_group(env, group_id).ok_or(ContractError::GroupNotFound)?;

    if admin != group.admin && admin != storage::get_admin(env) {
        return Err(ContractError::Unauthorized);
    }

    if group.status == GroupStatus::Completed {
        return Err(ContractError::GroupCompleted);
    }

    group.status = GroupStatus::Paused;
    storage::set_group(env, &group);

    env.events()
        .publish((crate::symbol_short!("grp_paus"),), group_id);

    Ok(())
}

pub fn resume_group(env: &Env, admin: Address, group_id: u64) -> Result<(), ContractError> {
    admin.require_auth();

    let mut group = storage::get_group(env, group_id).ok_or(ContractError::GroupNotFound)?;

    if admin != group.admin && admin != storage::get_admin(env) {
        return Err(ContractError::Unauthorized);
    }

    if group.status != GroupStatus::Paused {
        return Err(ContractError::GroupNotActive);
    }

    group.status = GroupStatus::Active;
    storage::set_group(env, &group);

    env.events()
        .publish((crate::symbol_short!("grp_resm"),), group_id);

    Ok(())
}

pub fn raise_dispute(
    env: &Env,
    member: Address,
    group_id: u64,
    reason: String,
) -> Result<(), ContractError> {
    member.require_auth();

    let mut group = storage::get_group(env, group_id).ok_or(ContractError::GroupNotFound)?;

    // Verify membership
    let mut is_member = false;
    for m in group.members.iter() {
        if m == member {
            is_member = true;
            break;
        }
    }
    if !is_member {
        return Err(ContractError::NotMember);
    }

    if group.status != GroupStatus::Active {
        return Err(ContractError::GroupNotActive);
    }

    let dispute = Dispute {
        raised_by: member.clone(),
        reason,
        raised_at: env.ledger().timestamp(),
    };

    group.status = GroupStatus::Disputed;
    storage::set_group(env, &group);
    storage::set_dispute(env, group_id, &dispute);
    storage::set_dispute_votes(
        env,
        group_id,
        &DisputeVotes {
            approvals: 0,
            rejections: 0,
            total_members: group.members.len(),
        },
    );

    env.events()
        .publish((crate::symbol_short!("dispute"),), (group_id, member));

    Ok(())
}

pub fn resolve_dispute(env: &Env, admin: Address, group_id: u64) -> Result<(), ContractError> {
    admin.require_auth();

    let mut group = storage::get_group(env, group_id).ok_or(ContractError::GroupNotFound)?;

    if admin != group.admin && admin != storage::get_admin(env) {
        return Err(ContractError::Unauthorized);
    }

    if group.status != GroupStatus::Disputed {
        return Err(ContractError::GroupNotActive);
    }

    group.status = GroupStatus::Active;
    storage::set_group(env, &group);
    storage::remove_dispute(env, group_id);
    storage::remove_dispute_votes(env, group_id);
    for m in group.members.iter() {
        storage::remove_dispute_vote(env, group_id, &m);
    }

    env.events()
        .publish((crate::symbol_short!("resolved"),), group_id);

    Ok(())
}

pub fn set_dispute_quorum(
    env: &Env,
    admin: Address,
    group_id: u64,
    quorum_bps: u32,
) -> Result<(), ContractError> {
    admin.require_auth();

    let group = storage::get_group(env, group_id).ok_or(ContractError::GroupNotFound)?;
    if admin != group.admin && admin != storage::get_admin(env) {
        return Err(ContractError::Unauthorized);
    }

    // Must be within (0%, 100%]
    if quorum_bps == 0 || quorum_bps > 10_000 {
        return Err(ContractError::InvalidAmount);
    }

    storage::set_dispute_quorum_bps(env, group_id, quorum_bps);
    Ok(())
}

pub fn vote_on_dispute(
    env: &Env,
    member: Address,
    group_id: u64,
    approve: bool,
) -> Result<(), ContractError> {
    member.require_auth();

    let mut group = storage::get_group(env, group_id).ok_or(ContractError::GroupNotFound)?;
    if group.status != GroupStatus::Disputed {
        return Err(ContractError::GroupNotActive);
    }

    // membership check
    let mut is_member = false;
    for m in group.members.iter() {
        if m == member {
            is_member = true;
            break;
        }
    }
    if !is_member {
        return Err(ContractError::NotMember);
    }

    if storage::has_dispute_vote(env, group_id, &member) {
        return Err(ContractError::AlreadyContributed);
    }

    let mut votes = storage::get_dispute_votes(env, group_id).unwrap_or(DisputeVotes {
        approvals: 0,
        rejections: 0,
        total_members: group.members.len(),
    });

    if approve {
        votes.approvals += 1;
    } else {
        votes.rejections += 1;
    }

    storage::set_dispute_vote(env, group_id, &member, approve);
    storage::set_dispute_votes(env, group_id, &votes);

    let quorum_bps = storage::get_dispute_quorum_bps(env, group_id) as u64;
    let approvals_bps = (votes.approvals as u64 * 10_000) / votes.total_members as u64;
    let rejections_bps = (votes.rejections as u64 * 10_000) / votes.total_members as u64;

    if approvals_bps >= quorum_bps {
        group.status = GroupStatus::Active;
        storage::set_group(env, &group);
        storage::remove_dispute(env, group_id);
        storage::remove_dispute_votes(env, group_id);
        for m in group.members.iter() {
            storage::remove_dispute_vote(env, group_id, &m);
        }
        env.events()
            .publish((crate::symbol_short!("dsp_appv"),), group_id);
    } else if rejections_bps >= quorum_bps {
        // rejected dispute keeps group active as well
        group.status = GroupStatus::Active;
        storage::set_group(env, &group);
        storage::remove_dispute(env, group_id);
        storage::remove_dispute_votes(env, group_id);
        for m in group.members.iter() {
            storage::remove_dispute_vote(env, group_id, &m);
        }
        env.events()
            .publish((crate::symbol_short!("dsp_rejt"),), group_id);
    }

    Ok(())
}

pub fn emergency_withdraw(env: &Env, admin: Address, group_id: u64) -> Result<(), ContractError> {
    admin.require_auth();

    let group = storage::get_group(env, group_id).ok_or(ContractError::GroupNotFound)?;

    // Only protocol admin can trigger emergency withdraw
    if admin != storage::get_admin(env) {
        return Err(ContractError::Unauthorized);
    }

    if group.status == GroupStatus::Completed {
        return Err(ContractError::GroupCompleted);
    }

    // Calculate remaining balance and distribute equally
    let token_client = soroban_sdk::token::Client::new(env, &group.token);
    let contract_addr = env.current_contract_address();
    let balance = token_client.balance(&contract_addr);

    if balance > 0 {
        let per_member = balance / group.members.len() as i128;
        if per_member > 0 {
            for member in group.members.iter() {
                token_client.transfer(&contract_addr, &member, &per_member);
            }
        }
    }

    let mut group = group;
    group.status = GroupStatus::Completed;
    storage::set_group(env, &group);

    env.events()
        .publish((crate::symbol_short!("emergenc"),), group_id);

    Ok(())
}

pub fn set_group_admin(
    env: &Env,
    current_admin: Address,
    group_id: u64,
    new_admin: Address,
) -> Result<(), ContractError> {
    current_admin.require_auth();

    let mut group = storage::get_group(env, group_id).ok_or(ContractError::GroupNotFound)?;

    if current_admin != group.admin {
        return Err(ContractError::Unauthorized);
    }

    group.admin = new_admin.clone();
    storage::set_group(env, &group);

    env.events()
        .publish((crate::symbol_short!("adm_chng"),), (group_id, new_admin));

    Ok(())
}
