use soroban_sdk::{Address, Env, String};

use crate::errors::ContractError;
use crate::storage;
use crate::types::{Dispute, GroupStatus};

pub fn pause_group(env: &Env, admin: Address, group_id: u64) -> Result<(), ContractError> {
    admin.require_auth();

    let mut group = storage::get_group(env, group_id).ok_or(ContractError::GroupNotFound)?;

    if admin != group.admin && admin != storage::get_admin(env) {
        return Err(ContractError::Unauthorized);
    }

    if group.status == GroupStatus::Completed {
        return Err(ContractError::GroupCompleted);
    }

    group.status = GroupStatus::Paused;
    storage::set_group(env, &group);

    env.events()
        .publish((crate::symbol_short!("grp_paus"),), group_id);

    Ok(())
}

pub fn resume_group(env: &Env, admin: Address, group_id: u64) -> Result<(), ContractError> {
    admin.require_auth();

    let mut group = storage::get_group(env, group_id).ok_or(ContractError::GroupNotFound)?;

    if admin != group.admin && admin != storage::get_admin(env) {
        return Err(ContractError::Unauthorized);
    }

    if group.status != GroupStatus::Paused {
        return Err(ContractError::GroupNotActive);
    }

    group.status = GroupStatus::Active;
    storage::set_group(env, &group);

    env.events()
        .publish((crate::symbol_short!("grp_resm"),), group_id);

    Ok(())
}

pub fn raise_dispute(
    env: &Env,
    member: Address,
    group_id: u64,
    reason: String,
) -> Result<(), ContractError> {
    member.require_auth();

    let mut group = storage::get_group(env, group_id).ok_or(ContractError::GroupNotFound)?;

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

    if group.status != GroupStatus::Active {
        return Err(ContractError::GroupNotActive);
    }

    let dispute = Dispute {
        raised_by: member.clone(),
        reason,
        raised_at: env.ledger().timestamp(),
    };

    group.status = GroupStatus::Disputed;
    storage::set_group(env, &group);
    storage::set_dispute(env, group_id, &dispute);

    env.events()
        .publish((crate::symbol_short!("dispute"),), (group_id, member));

    Ok(())
}

pub fn resolve_dispute(env: &Env, admin: Address, group_id: u64) -> Result<(), ContractError> {
    admin.require_auth();

    let mut group = storage::get_group(env, group_id).ok_or(ContractError::GroupNotFound)?;

    if admin != group.admin && admin != storage::get_admin(env) {
        return Err(ContractError::Unauthorized);
    }

    if group.status != GroupStatus::Disputed {
        return Err(ContractError::GroupNotActive);
    }

    group.status = GroupStatus::Active;
    storage::set_group(env, &group);
    storage::remove_dispute(env, group_id);

    env.events()
        .publish((crate::symbol_short!("resolved"),), group_id);

    Ok(())
}

pub fn emergency_withdraw(env: &Env, admin: Address, group_id: u64) -> Result<(), ContractError> {
    admin.require_auth();

    let group = storage::get_group(env, group_id).ok_or(ContractError::GroupNotFound)?;

    // Only protocol admin can trigger emergency withdraw
    if admin != storage::get_admin(env) {
        return Err(ContractError::Unauthorized);
    }

    if group.status == GroupStatus::Completed {
        return Err(ContractError::GroupCompleted);
    }

    let token_client = soroban_sdk::token::Client::new(env, &group.token);
    let contract_addr = env.current_contract_address();
    let balance = token_client.balance(&contract_addr);

    if balance > 0 {
        // Calculate total deposits (these will be forfeited on default)
        let deposits = storage::get_deposits(env, group_id);
        let mut total_deposits = 0i128;
        for member in group.members.iter() {
            if let Some(deposit) = deposits.get(member) {
                total_deposits += deposit;
            }
        }

        // Distributable balance = total balance - deposits (deposits are forfeited)
        let distributable = balance - total_deposits;
        
        if distributable > 0 {
            let per_member = distributable / group.members.len() as i128;
            if per_member > 0 {
                for member in group.members.iter() {
                    token_client.transfer(&contract_addr, &member, &per_member);
                }
            }
        }
        // Note: Deposits remain in contract (forfeited on emergency/default)
    }

    let mut group = group;
    group.status = GroupStatus::Completed;
    storage::set_group(env, &group);

    env.events()
        .publish((crate::symbol_short!("emergenc"),), group_id);

    Ok(())
}

pub fn set_group_admin(
    env: &Env,
    current_admin: Address,
    group_id: u64,
    new_admin: Address,
) -> Result<(), ContractError> {
    current_admin.require_auth();

    let mut group = storage::get_group(env, group_id).ok_or(ContractError::GroupNotFound)?;

    if current_admin != group.admin {
        return Err(ContractError::Unauthorized);
    }

    group.admin = new_admin.clone();
    storage::set_group(env, &group);

    env.events()
        .publish((crate::symbol_short!("adm_chng"),), (group_id, new_admin));

    Ok(())
}
