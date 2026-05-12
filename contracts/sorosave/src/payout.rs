use soroban_sdk::{Address, Env, Map, Vec};

use crate::errors::ContractError;
use crate::storage;
use crate::types::{GroupStatus, RoundInfo};

pub fn distribute_payout(env: &Env, group_id: u64) -> Result<(), ContractError> {
    let mut group = storage::get_group(env, group_id).ok_or(ContractError::GroupNotFound)?;

    if group.status != GroupStatus::Active {
        return Err(ContractError::GroupNotActive);
    }

    let round_info = storage::get_round(env, group_id, group.current_round)
        .ok_or(ContractError::RoundNotActive)?;

    if !round_info.is_complete {
        return Err(ContractError::RoundNotComplete);
    }

    // Transfer the pot to the round's recipient
    let token_client = soroban_sdk::token::Client::new(env, &group.token);
    token_client.transfer(
        &env.current_contract_address(),
        &round_info.recipient,
        &round_info.total_contributed,
    );

    env.events().publish(
        (crate::symbol_short!("payout"),),
        (
            group_id,
            round_info.recipient.clone(),
            round_info.total_contributed,
        ),
    );

    advance_round(env, group_id, &mut group)
}

pub fn request_early_payout(
    env: &Env,
    recipient: Address,
    group_id: u64,
) -> Result<(), ContractError> {
    recipient.require_auth();

    let mut group = storage::get_group(env, group_id).ok_or(ContractError::GroupNotFound)?;

    if group.status != GroupStatus::Active {
        return Err(ContractError::GroupNotActive);
    }

    let round_info = storage::get_round(env, group_id, group.current_round)
        .ok_or(ContractError::RoundNotActive)?;

    if round_info.is_complete {
        return Err(ContractError::RoundNotActive);
    }
    if recipient != round_info.recipient {
        return Err(ContractError::Unauthorized);
    }

    let required_amount = group.contribution_amount
        * group.members.len() as i128
        * group.early_payout_threshold_bps as i128
        / 10_000;
    if round_info.total_contributed < required_amount {
        return Err(ContractError::RoundNotComplete);
    }

    let token_client = soroban_sdk::token::Client::new(env, &group.token);
    token_client.transfer(
        &env.current_contract_address(),
        &recipient,
        &round_info.total_contributed,
    );

    env.events().publish(
        (crate::symbol_short!("earlypay"),),
        (group_id, recipient, round_info.total_contributed),
    );

    advance_round(env, group_id, &mut group)
}

pub fn set_early_payout_threshold(
    env: &Env,
    admin: Address,
    group_id: u64,
    threshold_bps: u32,
) -> Result<(), ContractError> {
    admin.require_auth();

    let mut group = storage::get_group(env, group_id).ok_or(ContractError::GroupNotFound)?;
    if admin != group.admin {
        return Err(ContractError::Unauthorized);
    }
    if threshold_bps == 0 || threshold_bps > 10_000 {
        return Err(ContractError::InvalidAmount);
    }

    group.early_payout_threshold_bps = threshold_bps;
    storage::set_group(env, &group);

    env.events().publish(
        (crate::symbol_short!("earlythr"),),
        (group_id, threshold_bps),
    );

    Ok(())
}

fn advance_round(
    env: &Env,
    group_id: u64,
    group: &mut crate::types::SavingsGroup,
) -> Result<(), ContractError> {
    // Advance to next round or complete the group
    if group.current_round >= group.total_rounds {
        group.status = GroupStatus::Completed;
        storage::set_group(env, group);

        env.events()
            .publish((crate::symbol_short!("grp_comp"),), group_id);
    } else {
        group.current_round += 1;
        let next_recipient = group.payout_order.get(group.current_round - 1).unwrap();

        let new_round = RoundInfo {
            round_number: group.current_round,
            recipient: next_recipient,
            contributions: Map::new(env),
            total_contributed: 0,
            is_complete: false,
            deadline: env.ledger().timestamp() + group.cycle_length,
        };

        storage::set_round(env, group_id, &new_round);
        storage::set_group(env, group);

        env.events().publish(
            (crate::symbol_short!("rnd_new"),),
            (group_id, group.current_round),
        );
    }

    Ok(())
}

pub fn get_payout_order(env: &Env, group_id: u64) -> Result<Vec<Address>, ContractError> {
    let group = storage::get_group(env, group_id).ok_or(ContractError::GroupNotFound)?;
    Ok(group.payout_order)
}

pub fn get_current_recipient(env: &Env, group_id: u64) -> Result<Address, ContractError> {
    let group = storage::get_group(env, group_id).ok_or(ContractError::GroupNotFound)?;

    if group.status != GroupStatus::Active {
        return Err(ContractError::GroupNotActive);
    }

    let round_info = storage::get_round(env, group_id, group.current_round)
        .ok_or(ContractError::RoundNotActive)?;

    Ok(round_info.recipient)
}
