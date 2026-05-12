use soroban_sdk::{Address, Env};

use crate::errors::ContractError;
use crate::storage;
use crate::types::{GroupStatus, RoundInfo};

pub fn contribute(env: &Env, member: Address, group_id: u64) -> Result<(), ContractError> {
    member.require_auth();

    let group = storage::get_group(env, group_id).ok_or(ContractError::GroupNotFound)?;

    if group.status != GroupStatus::Active {
        return Err(ContractError::GroupNotActive);
    }

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

    let mut round_info = storage::get_round(env, group_id, group.current_round)
        .ok_or(ContractError::RoundNotActive)?;

    if round_info.is_complete {
        return Err(ContractError::RoundNotActive);
    }

    // Check if already contributed this round
    if round_info.contributions.contains_key(member.clone()) {
        return Err(ContractError::AlreadyContributed);
    }

    // Transfer tokens from member to this contract
    let token_client = soroban_sdk::token::Client::new(env, &group.token);
    token_client.transfer(
        &member,
        &env.current_contract_address(),
        &group.contribution_amount,
    );

    // Record contribution
    round_info.contributions.set(member.clone(), true);
    round_info.total_contributed += group.contribution_amount;

    // Check if all members have contributed
    if round_info.contributions.len() == group.members.len() {
        round_info.is_complete = true;
    }

    storage::set_round(env, group_id, &round_info);

    env.events().publish(
        (crate::symbol_short!("contrib"),),
        (group_id, member, group.contribution_amount),
    );

    Ok(())
}

pub fn get_round_status(env: &Env, group_id: u64, round: u32) -> Result<RoundInfo, ContractError> {
    storage::get_round(env, group_id, round).ok_or(ContractError::RoundNotActive)
}

pub fn has_contributed(
    env: &Env,
    member: Address,
    group_id: u64,
    round: u32,
) -> Result<bool, ContractError> {
    let round_info =
        storage::get_round(env, group_id, round).ok_or(ContractError::RoundNotActive)?;
    Ok(round_info.contributions.contains_key(member))
}

pub fn set_penalty_rate(
    env: &Env,
    admin: Address,
    group_id: u64,
    penalty_rate_bps: u32,
) -> Result<(), ContractError> {
    admin.require_auth();

    let mut group = storage::get_group(env, group_id).ok_or(ContractError::GroupNotFound)?;
    if admin != group.admin {
        return Err(ContractError::Unauthorized);
    }
    if penalty_rate_bps > 10_000 {
        return Err(ContractError::InvalidAmount);
    }

    group.penalty_rate_bps = penalty_rate_bps;
    storage::set_group(env, &group);

    env.events().publish(
        (crate::symbol_short!("pen_rate"),),
        (group_id, penalty_rate_bps),
    );

    Ok(())
}

pub fn apply_missed_penalty(
    env: &Env,
    admin: Address,
    group_id: u64,
    member: Address,
) -> Result<i128, ContractError> {
    admin.require_auth();

    let group = storage::get_group(env, group_id).ok_or(ContractError::GroupNotFound)?;
    if admin != group.admin {
        return Err(ContractError::Unauthorized);
    }
    if group.status != GroupStatus::Active {
        return Err(ContractError::GroupNotActive);
    }
    if group.penalty_rate_bps == 0 {
        return Err(ContractError::InvalidAmount);
    }

    let round_info = storage::get_round(env, group_id, group.current_round)
        .ok_or(ContractError::RoundNotActive)?;
    if env.ledger().timestamp() <= round_info.deadline {
        return Err(ContractError::RoundNotActive);
    }
    if round_info.contributions.contains_key(member.clone()) {
        return Err(ContractError::AlreadyContributed);
    }
    if !is_group_member(&group.members, &member) {
        return Err(ContractError::NotMember);
    }

    let penalty = group.contribution_amount * group.penalty_rate_bps as i128 / 10_000;
    let current_penalty = storage::get_member_penalty(env, group_id, &member);
    storage::set_member_penalty(env, group_id, &member, current_penalty + penalty);

    env.events().publish(
        (crate::symbol_short!("penalty"),),
        (group_id, member, penalty),
    );

    Ok(penalty)
}

pub fn get_member_penalty(env: &Env, group_id: u64, member: Address) -> i128 {
    storage::get_member_penalty(env, group_id, &member)
}

fn is_group_member(members: &soroban_sdk::Vec<Address>, member: &Address) -> bool {
    for current in members.iter() {
        if current == *member {
            return true;
        }
    }

    false
}
