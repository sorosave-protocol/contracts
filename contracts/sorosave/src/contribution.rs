use soroban_sdk::{Address, Env};

use crate::errors::ContractError;
use crate::storage;
use crate::types::{GroupStatus, RoundInfo};

pub fn contribute(env: &Env, member: Address, group_id: u64) -> Result<(), ContractError> {
    let group = storage::get_group(env, group_id).ok_or(ContractError::GroupNotFound)?;
    contribute_partial(env, member, group_id, group.contribution_amount)
}

pub fn contribute_partial(
    env: &Env,
    member: Address,
    group_id: u64,
    amount: i128,
) -> Result<(), ContractError> {
    member.require_auth();
    if amount <= 0 {
        return Err(ContractError::InvalidAmount);
    }

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

    if round_info.contributions.contains_key(member.clone()) {
        return Err(ContractError::AlreadyContributed);
    }
    let current_progress = round_info
        .contribution_amounts
        .get(member.clone())
        .unwrap_or(0);
    let remaining = group.contribution_amount - current_progress;
    if amount > remaining {
        return Err(ContractError::InvalidAmount);
    }

    // Transfer tokens from member to this contract
    let token_client = soroban_sdk::token::Client::new(env, &group.token);
    token_client.transfer(&member, &env.current_contract_address(), &amount);

    // Record contribution
    let new_progress = current_progress + amount;
    round_info
        .contribution_amounts
        .set(member.clone(), new_progress);
    round_info.total_contributed += amount;
    if new_progress == group.contribution_amount {
        round_info.contributions.set(member.clone(), true);
    }

    // Check if all members have contributed
    if round_info.contributions.len() == group.members.len() {
        round_info.is_complete = true;
    }

    storage::set_round(env, group_id, &round_info);

    env.events().publish(
        (crate::symbol_short!("contrib"),),
        (group_id, member, amount, new_progress),
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

pub fn get_member_contribution_progress(
    env: &Env,
    member: Address,
    group_id: u64,
    round: u32,
) -> Result<i128, ContractError> {
    let round_info =
        storage::get_round(env, group_id, round).ok_or(ContractError::RoundNotActive)?;
    Ok(round_info.contribution_amounts.get(member).unwrap_or(0))
}
