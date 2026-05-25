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

    // Split the completed pot into protocol fee and recipient payout.
    let token_client = soroban_sdk::token::Client::new(env, &group.token);
    let contract_addr = env.current_contract_address();
    let fee_bps = storage::get_protocol_fee_bps(env);
    let fee_denominator = storage::MAX_PROTOCOL_FEE_BPS as i128;
    let fee_bps_i128 = fee_bps as i128;
    let fee_amount = (round_info.total_contributed / fee_denominator) * fee_bps_i128
        + (round_info.total_contributed % fee_denominator) * fee_bps_i128 / fee_denominator;
    let payout_amount = round_info.total_contributed - fee_amount;

    if fee_amount > 0 {
        let treasury = storage::get_protocol_treasury(env);
        token_client.transfer(&contract_addr, &treasury, &fee_amount);

        env.events().publish(
            (crate::symbol_short!("fee_paid"),),
            (group_id, treasury, fee_amount),
        );
    }

    if payout_amount > 0 {
        token_client.transfer(&contract_addr, &round_info.recipient, &payout_amount);
    }

    env.events().publish(
        (crate::symbol_short!("payout"),),
        (group_id, round_info.recipient.clone(), payout_amount),
    );

    // Advance to next round or complete the group
    if group.current_round >= group.total_rounds {
        group.status = GroupStatus::Completed;
        storage::set_group(env, &group);

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
        storage::set_group(env, &group);

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
