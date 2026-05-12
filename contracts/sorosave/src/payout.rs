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

    // Advance to next round, restart the group, or complete the group.
    if group.current_round >= group.total_rounds {
        if group.auto_restart {
            restart_group(env, &mut group, group_id);
        } else {
            group.status = GroupStatus::Completed;
            storage::set_group(env, &group);

            env.events()
                .publish((crate::symbol_short!("grp_comp"),), group_id);
        }
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
        storage::set_group(env, &group);

        env.events().publish(
            (crate::symbol_short!("rnd_new"),),
            (group_id, group.current_round),
        );
    }

    Ok(())
}

fn restart_group(env: &Env, group: &mut crate::types::SavingsGroup, group_id: u64) {
    let mut next_members = Vec::new(env);
    for member in group.members.iter() {
        let mut opted_out = false;
        for opt_out in group.next_cycle_opt_outs.iter() {
            if opt_out == member {
                opted_out = true;
                break;
            }
        }
        if !opted_out {
            next_members.push_back(member);
        }
    }

    if next_members.len() < 2 {
        group.status = GroupStatus::Completed;
        group.next_cycle_opt_outs = Vec::new(env);
        storage::set_group(env, group);
        env.events()
            .publish((crate::symbol_short!("grp_comp"),), group_id);
        return;
    }

    group.members = next_members.clone();
    group.payout_order = rotate_payout_order(env, &next_members);
    group.next_cycle_opt_outs = Vec::new(env);
    group.current_round = 1;
    group.total_rounds = group.members.len();
    group.status = GroupStatus::Active;

    let first_recipient = group.payout_order.get(0).unwrap();
    let first_round = RoundInfo {
        round_number: 1,
        recipient: first_recipient,
        contributions: Map::new(env),
        total_contributed: 0,
        is_complete: false,
        deadline: env.ledger().timestamp() + group.cycle_length,
    };

    storage::set_round(env, group_id, &first_round);
    storage::set_group(env, group);

    env.events().publish(
        (crate::symbol_short!("grp_rstr"),),
        (group_id, group.total_rounds),
    );
}

fn rotate_payout_order(env: &Env, members: &Vec<Address>) -> Vec<Address> {
    let mut payout_order = Vec::new(env);
    if members.is_empty() {
        return payout_order;
    }

    for index in 1..members.len() {
        if let Some(member) = members.get(index) {
            payout_order.push_back(member);
        }
    }
    if let Some(first_member) = members.get(0) {
        payout_order.push_back(first_member);
    }

    payout_order
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
