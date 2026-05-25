use soroban_sdk::{Address, Env};

use crate::errors::ContractError;
use crate::storage;
use crate::types::{GroupStatus, RoundInfo, SavingsGroup};

fn is_token_accepted(group: &SavingsGroup, token: &Address) -> bool {
    for accepted_token in group.accepted_tokens.iter() {
        if accepted_token == token.clone() {
            return true;
        }
    }
    false
}

pub fn contribute(env: &Env, member: Address, group_id: u64) -> Result<(), ContractError> {
    let group = storage::get_group(env, group_id).ok_or(ContractError::GroupNotFound)?;
    contribute_with_token(env, member, group_id, group.token)
}

pub fn contribute_with_token(
    env: &Env,
    member: Address,
    group_id: u64,
    token: Address,
) -> Result<(), ContractError> {
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

    if !is_token_accepted(&group, &token) {
        return Err(ContractError::InvalidAmount);
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
    let token_client = soroban_sdk::token::Client::new(env, &token);
    token_client.transfer(
        &member,
        &env.current_contract_address(),
        &group.contribution_amount,
    );

    // Record contribution
    round_info.contributions.set(member.clone(), true);
    round_info.total_contributed += group.contribution_amount;
    let token_total = round_info
        .token_contributions
        .get(token.clone())
        .unwrap_or(0)
        + group.contribution_amount;
    round_info
        .token_contributions
        .set(token.clone(), token_total);

    // Check if all members have contributed
    if round_info.contributions.len() == group.members.len() {
        round_info.is_complete = true;
    }

    storage::set_round(env, group_id, &round_info);

    env.events().publish(
        (crate::symbol_short!("contrib"),),
        (group_id, member, token, group.contribution_amount),
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
