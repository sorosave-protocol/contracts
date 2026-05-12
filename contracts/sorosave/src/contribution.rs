use soroban_sdk::{Address, Env};

use crate::errors::ContractError;
use crate::storage;
use crate::types::{GroupStatus, RoundInfo};

pub fn contribute(env: &Env, member: Address, group_id: u64) -> Result<(), ContractError> {
    member.require_auth();
    contribute_for_member(env, member.clone(), member, group_id)
}

pub fn set_delegate(
    env: &Env,
    member: Address,
    delegate: Address,
    group_id: u64,
) -> Result<(), ContractError> {
    member.require_auth();
    let group = storage::get_group(env, group_id).ok_or(ContractError::GroupNotFound)?;
    ensure_member(&group.members, &member)?;

    storage::set_delegate(env, group_id, &member, &delegate);
    env.events().publish(
        (crate::symbol_short!("delegate"),),
        (group_id, member, delegate),
    );

    Ok(())
}

pub fn revoke_delegate(env: &Env, member: Address, group_id: u64) -> Result<(), ContractError> {
    member.require_auth();
    let group = storage::get_group(env, group_id).ok_or(ContractError::GroupNotFound)?;
    ensure_member(&group.members, &member)?;

    storage::remove_delegate(env, group_id, &member);
    env.events()
        .publish((crate::symbol_short!("del_rev"),), (group_id, member));

    Ok(())
}

pub fn get_delegate(env: &Env, member: Address, group_id: u64) -> Option<Address> {
    storage::get_delegate(env, group_id, &member)
}

pub fn contribute_for(
    env: &Env,
    delegate: Address,
    member: Address,
    group_id: u64,
) -> Result<(), ContractError> {
    delegate.require_auth();
    let active_delegate =
        storage::get_delegate(env, group_id, &member).ok_or(ContractError::Unauthorized)?;
    if active_delegate != delegate {
        return Err(ContractError::Unauthorized);
    }

    contribute_for_member(env, delegate, member, group_id)
}

fn contribute_for_member(
    env: &Env,
    payer: Address,
    member: Address,
    group_id: u64,
) -> Result<(), ContractError> {
    let group = storage::get_group(env, group_id).ok_or(ContractError::GroupNotFound)?;

    if group.status != GroupStatus::Active {
        return Err(ContractError::GroupNotActive);
    }

    ensure_member(&group.members, &member)?;

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
        &payer,
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

fn ensure_member(
    members: &soroban_sdk::Vec<Address>,
    member: &Address,
) -> Result<(), ContractError> {
    for group_member in members.iter() {
        if group_member == *member {
            return Ok(());
        }
    }
    Err(ContractError::NotMember)
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
