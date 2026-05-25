use soroban_sdk::{Address, Env, Vec};

use crate::errors::ContractError;
use crate::storage;
use crate::types::{GroupStatus, RoundInfo, SavingsGroup};

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
    if round_info.misses.contains_key(member.clone()) {
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
    storage::clear_consecutive_misses(env, group_id, &member);

    refresh_round_completion(&group, &mut round_info);

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

pub fn get_consecutive_misses(
    env: &Env,
    group_id: u64,
    member: Address,
) -> Result<u32, ContractError> {
    let group = storage::get_group(env, group_id).ok_or(ContractError::GroupNotFound)?;
    ensure_member(&group, &member)?;
    Ok(storage::get_consecutive_misses(env, group_id, &member))
}

pub fn record_missed_contribution(
    env: &Env,
    admin: Address,
    group_id: u64,
    member: Address,
) -> Result<(), ContractError> {
    admin.require_auth();

    let mut group = storage::get_group(env, group_id).ok_or(ContractError::GroupNotFound)?;

    if admin != group.admin && admin != storage::get_admin(env) {
        return Err(ContractError::Unauthorized);
    }

    if group.status != GroupStatus::Active {
        return Err(ContractError::GroupNotActive);
    }

    ensure_member(&group, &member)?;

    let mut round_info = storage::get_round(env, group_id, group.current_round)
        .ok_or(ContractError::RoundNotActive)?;

    if env.ledger().timestamp() < round_info.deadline {
        return Err(ContractError::DeadlineNotReached);
    }

    if round_info.contributions.contains_key(member.clone()) {
        return Err(ContractError::AlreadyContributed);
    }
    if round_info.misses.contains_key(member.clone()) {
        return Err(ContractError::AlreadyContributed);
    }

    let misses = storage::get_consecutive_misses(env, group_id, &member) + 1;
    storage::set_consecutive_misses(env, group_id, &member, misses);
    round_info.misses.set(member.clone(), true);

    env.events().publish(
        (crate::symbol_short!("miss_rec"),),
        (group_id, member.clone(), misses),
    );

    if misses >= group.max_consecutive_misses {
        remove_defaulted_member(env, &mut group, &mut round_info, &member)?;
        round_info.misses.remove(member.clone());
        storage::clear_consecutive_misses(env, group_id, &member);
        storage::remove_member_group(env, &member, group_id);

        env.events()
            .publish((crate::symbol_short!("mbr_rmvd"),), (group_id, member));
    }

    refresh_round_completion(&group, &mut round_info);

    storage::set_round(env, group_id, &round_info);
    storage::set_group(env, &group);

    Ok(())
}

fn ensure_member(group: &SavingsGroup, member: &Address) -> Result<(), ContractError> {
    for m in group.members.iter() {
        if m == *member {
            return Ok(());
        }
    }
    Err(ContractError::NotMember)
}

fn refresh_round_completion(group: &SavingsGroup, round_info: &mut RoundInfo) {
    if round_info.contributions.len() + round_info.misses.len() >= group.members.len() {
        round_info.is_complete = true;
    }
}

fn remove_defaulted_member(
    env: &Env,
    group: &mut SavingsGroup,
    round_info: &mut RoundInfo,
    member: &Address,
) -> Result<(), ContractError> {
    let mut remaining_members = Vec::new(env);
    for m in group.members.iter() {
        if m != *member {
            remaining_members.push_back(m);
        }
    }

    if remaining_members.is_empty() {
        group.members = remaining_members;
        group.status = GroupStatus::Completed;
        round_info.is_complete = true;
        return Ok(());
    }

    let replacement = choose_replacement_for_slot(&group.payout_order, &remaining_members, member);
    let mut redistributed_order = Vec::new(env);
    for recipient in group.payout_order.iter() {
        if recipient == *member {
            redistributed_order.push_back(replacement.clone());
        } else {
            redistributed_order.push_back(recipient);
        }
    }

    if round_info.recipient == *member {
        round_info.recipient = replacement;
    }

    group.members = remaining_members;
    group.payout_order = redistributed_order;

    Ok(())
}

fn choose_replacement_for_slot(
    payout_order: &Vec<Address>,
    remaining_members: &Vec<Address>,
    removed_member: &Address,
) -> Address {
    let mut removed_index = 0;
    for i in 0..payout_order.len() {
        if payout_order.get(i).unwrap() == *removed_member {
            removed_index = i;
            break;
        }
    }

    let replacement_index = if removed_index < remaining_members.len() {
        removed_index
    } else {
        remaining_members.len() - 1
    };

    remaining_members.get(replacement_index).unwrap()
}
