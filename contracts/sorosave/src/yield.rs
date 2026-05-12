use soroban_sdk::Env;

use crate::errors::ContractError;
use crate::storage;
use crate::types::{RoundInfo, SavingsGroup, YieldPosition};

pub trait YieldSource {
    fn deposit(
        env: &Env,
        group_id: u64,
        round: u32,
        amount: i128,
        rate_bps: u32,
    ) -> Result<(), ContractError>;

    fn withdraw(env: &Env, group_id: u64, round: u32) -> Result<i128, ContractError>;
}

pub struct FixedRateYieldSource;

impl YieldSource for FixedRateYieldSource {
    fn deposit(
        env: &Env,
        group_id: u64,
        round: u32,
        amount: i128,
        rate_bps: u32,
    ) -> Result<(), ContractError> {
        if amount <= 0 || rate_bps == 0 {
            return Ok(());
        }

        let position = YieldPosition {
            principal: amount,
            earned_yield: amount * rate_bps as i128 / 10_000,
            deposited_at: env.ledger().timestamp(),
        };
        storage::set_yield_position(env, group_id, round, &position);

        env.events().publish(
            (crate::symbol_short!("yielddep"),),
            (group_id, round, amount),
        );

        Ok(())
    }

    fn withdraw(env: &Env, group_id: u64, round: u32) -> Result<i128, ContractError> {
        let Some(position) = storage::get_yield_position(env, group_id, round) else {
            return Ok(0);
        };

        storage::remove_yield_position(env, group_id, round);
        let amount = position.principal + position.earned_yield;

        env.events().publish(
            (crate::symbol_short!("yieldwd"),),
            (group_id, round, amount),
        );

        Ok(amount)
    }
}

pub fn deposit_idle_funds(
    env: &Env,
    group: &SavingsGroup,
    round_info: &RoundInfo,
) -> Result<(), ContractError> {
    FixedRateYieldSource::deposit(
        env,
        group.id,
        round_info.round_number,
        round_info.total_contributed,
        group.yield_rate_bps,
    )
}

pub fn withdraw_for_payout(
    env: &Env,
    group: &SavingsGroup,
    round_info: &RoundInfo,
) -> Result<i128, ContractError> {
    let yielded_amount = FixedRateYieldSource::withdraw(env, group.id, round_info.round_number)?;
    if yielded_amount > 0 {
        Ok(yielded_amount)
    } else {
        Ok(round_info.total_contributed)
    }
}
