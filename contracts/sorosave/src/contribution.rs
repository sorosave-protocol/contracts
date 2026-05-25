use soroban_sdk::{Address, Env};

use crate::errors::ContractError;
use crate::storage;
use crate::types::{ContributionType, GroupStatus, RoundInfo, SavingsGroup};

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

    let contribution_amount = contribution_amount_for_member(&group, &member)?;

    // Transfer tokens from member to this contract
    let token_client = soroban_sdk::token::Client::new(env, &group.token);
    token_client.transfer(
        &member,
        &env.current_contract_address(),
        &contribution_amount,
    );

    // Record contribution
    round_info.contributions.set(member.clone(), true);
    round_info.total_contributed += contribution_amount;

    // Check if all members have contributed
    if round_info.contributions.len() == group.members.len() {
        round_info.is_complete = true;
    }

    storage::set_round(env, group_id, &round_info);

    env.events().publish(
        (crate::symbol_short!("contrib"),),
        (group_id, member, contribution_amount),
    );

    Ok(())
}

fn contribution_amount_for_member(
    group: &SavingsGroup,
    member: &Address,
) -> Result<i128, ContractError> {
    match &group.contribution_type {
        ContributionType::Fixed => Ok(group.contribution_amount),
        ContributionType::Percentage => {
            let base_amount = group
                .member_base_amounts
                .get(member.clone())
                .ok_or(ContractError::InvalidAmount)?;
            let contribution_amount = base_amount
                .checked_mul(group.contribution_amount)
                .ok_or(ContractError::InvalidAmount)?
                / 10_000;

            if contribution_amount <= 0 {
                return Err(ContractError::InvalidAmount);
            }

            Ok(contribution_amount)
        }
    }
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
