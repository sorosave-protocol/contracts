use soroban_sdk::{Address, Env, Map, String, Vec};

use crate::errors::ContractError;
use crate::storage;
use crate::types::{GroupStatus, RoundInfo, SavingsGroup};

pub fn create_group(
    env: &Env,
    admin: Address,
    name: String,
    token: Address,
    contribution_amount: i128,
    cycle_length: u64,
    max_members: u32,
) -> Result<u64, ContractError> {
    admin.require_auth();

    if contribution_amount <= 0 {
        return Err(ContractError::InvalidAmount);
    }
    if max_members < 2 {
        return Err(ContractError::InsufficientMembers);
    }

    let group_id = storage::get_group_counter(env) + 1;
    storage::set_group_counter(env, group_id);

    let mut members = Vec::new(env);
    members.push_back(admin.clone());

    let group = SavingsGroup {
        id: group_id,
        name,
        admin: admin.clone(),
        token,
        contribution_amount,
        cycle_length,
        max_members,
        members,
        payout_order: Vec::new(env),
        current_round: 0,
        total_rounds: 0,
        status: GroupStatus::Forming,
        created_at: env.ledger().timestamp(),
    };

    storage::set_group(env, &group);
    storage::add_member_group(env, &admin, group_id);

    env.events()
        .publish((crate::symbol_short!("grp_creat"),), group_id);

    Ok(group_id)
}

pub fn join_group(
    env: &Env, 
    member: Address, 
    group_id: u64, 
    referred_by: Option<Address>
) -> Result<(), ContractError> {
    member.require_auth();

    let mut group = storage::get_group(env, group_id)?;

    if group.status != GroupStatus::Forming {
        return Err(ContractError::GroupNotOpen);
    }

    if group.members.len() >= group.max_members {
        return Err(ContractError::GroupFull);
    }

    for i in 0..group.members.len() {
        if group.members.get(i).unwrap() == member {
            return Err(ContractError::AlreadyMember);
        }
    }

    // Handle referral if provided
    if let Some(referrer) = &referred_by {
        // Verify referrer is a member of the group
        let mut referrer_is_member = false;
        for i in 0..group.members.len() {
            if group.members.get(i).unwrap() == *referrer {
                referrer_is_member = true;
                break;
            }
        }

        if !referrer_is_member {
            return Err(ContractError::InvalidReferrer);
        }

        // Track the referral
        storage::add_referral(env, referrer, &member, group_id);

        // Emit referral event
        env.events().publish(
            (crate::symbol_short!("referral"),), 
            (referrer.clone(), member.clone(), group_id)
        );
    }

    group.members.push_back(member.clone());
    storage::set_group(env, &group);
    storage::add_member_group(env, &member, group_id);

    env.events()
        .publish((crate::symbol_short!("mem_join"),), (member, group_id));

    Ok(())
}

pub fn get_referral_count(env: &Env, referrer: Address, group_id: u64) -> u32 {
    storage::get_referral_count(env, &referrer, group_id)
}

pub fn start_group(env: &Env, admin: Address, group_id: u64) -> Result<(), ContractError> {
    admin.require_auth();

    let mut group = storage::get_group(env, group_id)?;

    if group.admin != admin {
        return Err(ContractError::Unauthorized);
    }

    if group.status != GroupStatus::Forming {
        return Err(ContractError::InvalidGroupStatus);
    }

    if group.members.len() < 2 {
        return Err(ContractError::InsufficientMembers);
    }

    group.status = GroupStatus::Active;
    group.total_rounds = group.members.len() as u32;
    group.current_round = 1;

    // Initialize payout order (can be randomized or based on join order)
    group.payout_order = group.members.clone();

    storage::set_group(env, &group);

    env.events()
        .publish((crate::symbol_short!("grp_start"),), group_id);

    Ok(())
}