use soroban_sdk::{Address, Env, Map, Vec};

use crate::errors::ContractError;
use crate::storage;
use crate::types::{GroupStatus, RoundInfo, SavingsGroup};

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

    // Advance to next round, restart a recurring group, or complete it.
    if group.current_round >= group.total_rounds {
        if group.auto_restart {
            restart_or_complete_group(env, &mut group);
        } else {
            group.status = GroupStatus::Completed;
            storage::set_group(env, &group);

            env.events()
                .publish((crate::symbol_short!("grp_comp"),), group_id);
        }
    } else {
        group.current_round += 1;
        let new_round = build_round(env, &group, group.current_round);

        storage::set_round(env, group_id, &new_round);
        storage::set_group(env, &group);

        env.events().publish(
            (crate::symbol_short!("rnd_new"),),
            (group_id, group.current_round),
        );
    }

    Ok(())
}

fn restart_or_complete_group(env: &Env, group: &mut SavingsGroup) {
    let next_members = remaining_members(env, group);
    let next_order = rotated_payout_order(env, group, &next_members);

    remove_opted_out_members(env, group);
    group.restart_opt_outs = Vec::new(env);
    group.members = next_members;
    group.payout_order = next_order;

    if group.members.len() < 2 {
        group.status = GroupStatus::Completed;
        group.current_round = group.members.len();
        group.total_rounds = group.members.len();
        storage::set_group(env, group);

        env.events()
            .publish((crate::symbol_short!("grp_comp"),), group.id);
        return;
    }

    group.current_round = 1;
    group.total_rounds = group.members.len();
    group.status = GroupStatus::Active;

    let new_round = build_round(env, group, 1);
    storage::set_round(env, group.id, &new_round);
    storage::set_group(env, group);

    env.events()
        .publish((crate::symbol_short!("grp_rstr"),), group.id);
    env.events().publish(
        (crate::symbol_short!("rnd_new"),),
        (group.id, group.current_round),
    );
}

fn build_round(env: &Env, group: &SavingsGroup, round_number: u32) -> RoundInfo {
    let recipient = group.payout_order.get(round_number - 1).unwrap();

    RoundInfo {
        round_number,
        recipient,
        contributions: Map::new(env),
        total_contributed: 0,
        is_complete: false,
        deadline: env.ledger().timestamp() + group.cycle_length,
    }
}

fn remaining_members(env: &Env, group: &SavingsGroup) -> Vec<Address> {
    let mut members = Vec::new(env);
    for member in group.members.iter() {
        if !has_opted_out(group, &member) {
            members.push_back(member);
        }
    }

    members
}

fn rotated_payout_order(
    env: &Env,
    group: &SavingsGroup,
    next_members: &Vec<Address>,
) -> Vec<Address> {
    let mut order = Vec::new(env);

    if group.payout_order.is_empty() {
        return next_members.clone();
    }

    for offset in 1..=group.payout_order.len() {
        let index = offset % group.payout_order.len();
        let member = group.payout_order.get(index).unwrap();
        if is_next_member(next_members, &member) {
            order.push_back(member);
        }
    }

    order
}

fn remove_opted_out_members(env: &Env, group: &SavingsGroup) {
    for member in group.restart_opt_outs.iter() {
        storage::remove_member_group(env, &member, group.id);
    }
}

fn has_opted_out(group: &SavingsGroup, member: &Address) -> bool {
    for opted_out in group.restart_opt_outs.iter() {
        if opted_out == *member {
            return true;
        }
    }

    false
}

fn is_next_member(members: &Vec<Address>, member: &Address) -> bool {
    for next_member in members.iter() {
        if next_member == *member {
            return true;
        }
    }

    false
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
