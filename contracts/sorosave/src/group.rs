use soroban_sdk::{Address, Env, Map, Vec};

use crate::errors::ContractError;
use crate::storage;
use crate::types::{CreateGroupConfig, GroupStatus, RoundInfo, SavingsGroup};

pub fn create_group(
    env: &Env,
    admin: Address,
    config: CreateGroupConfig,
) -> Result<u64, ContractError> {
    admin.require_auth();

    if config.contribution_amount <= 0 {
        return Err(ContractError::InvalidAmount);
    }
    if config.max_members < 2 {
        return Err(ContractError::InsufficientMembers);
    }

    let group_id = storage::get_group_counter(env) + 1;
    storage::set_group_counter(env, group_id);

    let mut members = Vec::new(env);
    members.push_back(admin.clone());

    let group = SavingsGroup {
        id: group_id,
        name: config.name,
        admin: admin.clone(),
        token: config.token,
        contribution_amount: config.contribution_amount,
        cycle_length: config.cycle_length,
        max_members: config.max_members,
        randomize_order: config.randomize_order,
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

pub fn join_group(env: &Env, member: Address, group_id: u64) -> Result<(), ContractError> {
    member.require_auth();

    let mut group = storage::get_group(env, group_id).ok_or(ContractError::GroupNotFound)?;

    if group.status != GroupStatus::Forming {
        return Err(ContractError::GroupNotForming);
    }

    if group.members.len() >= group.max_members {
        return Err(ContractError::GroupFull);
    }

    // Check if already a member
    for m in group.members.iter() {
        if m == member {
            return Err(ContractError::AlreadyMember);
        }
    }

    group.members.push_back(member.clone());
    storage::set_group(env, &group);
    storage::add_member_group(env, &member, group_id);

    env.events()
        .publish((crate::symbol_short!("grp_join"),), (group_id, member));

    Ok(())
}

pub fn leave_group(env: &Env, member: Address, group_id: u64) -> Result<(), ContractError> {
    member.require_auth();

    let mut group = storage::get_group(env, group_id).ok_or(ContractError::GroupNotFound)?;

    if group.status != GroupStatus::Forming {
        return Err(ContractError::GroupNotForming);
    }

    // Admin cannot leave their own group
    if member == group.admin {
        return Err(ContractError::Unauthorized);
    }

    let mut found = false;
    let mut new_members = Vec::new(env);
    for m in group.members.iter() {
        if m == member {
            found = true;
        } else {
            new_members.push_back(m);
        }
    }

    if !found {
        return Err(ContractError::NotMember);
    }

    group.members = new_members;
    storage::set_group(env, &group);
    storage::remove_member_group(env, &member, group_id);

    env.events()
        .publish((crate::symbol_short!("grp_leav"),), (group_id, member));

    Ok(())
}

pub fn start_group(env: &Env, admin: Address, group_id: u64) -> Result<(), ContractError> {
    admin.require_auth();

    let mut group = storage::get_group(env, group_id).ok_or(ContractError::GroupNotFound)?;

    if admin != group.admin {
        return Err(ContractError::Unauthorized);
    }

    if group.status != GroupStatus::Forming {
        return Err(ContractError::GroupNotForming);
    }

    if group.members.len() < 2 {
        return Err(ContractError::InsufficientMembers);
    }

    group.payout_order = if group.randomize_order {
        deterministic_shuffle(env, group_id, &group.members)
    } else {
        group.members.clone()
    };
    group.total_rounds = group.members.len();
    group.current_round = 1;
    group.status = GroupStatus::Active;

    // Initialize first round
    let first_recipient = group.payout_order.get(0).unwrap();
    let round_info = RoundInfo {
        round_number: 1,
        recipient: first_recipient,
        contributions: Map::new(env),
        total_contributed: 0,
        is_complete: false,
        deadline: env.ledger().timestamp() + group.cycle_length,
    };

    storage::set_round(env, group_id, &round_info);
    storage::set_group(env, &group);

    env.events()
        .publish((crate::symbol_short!("grp_strt"),), group_id);

    Ok(())
}

pub fn get_group(env: &Env, group_id: u64) -> Result<SavingsGroup, ContractError> {
    storage::get_group(env, group_id).ok_or(ContractError::GroupNotFound)
}

pub fn get_member_groups(env: &Env, member: Address) -> Vec<u64> {
    storage::get_member_groups(env, &member)
}

fn deterministic_shuffle(env: &Env, group_id: u64, members: &Vec<Address>) -> Vec<Address> {
    let mut remaining = members.clone();
    let mut shuffled = Vec::new(env);
    let mut seed = env.ledger().timestamp() ^ ((env.ledger().sequence() as u64) << 32) ^ group_id;

    while !remaining.is_empty() {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let index = (seed % remaining.len() as u64) as u32;
        shuffled.push_back(remaining.get(index).unwrap());

        let mut next_remaining = Vec::new(env);
        for i in 0..remaining.len() {
            if i != index {
                next_remaining.push_back(remaining.get(i).unwrap());
            }
        }
        remaining = next_remaining;
    }

    if members.len() > 1 && has_same_order(members, &shuffled) {
        reverse_order(env, members)
    } else {
        shuffled
    }
}

fn has_same_order(left: &Vec<Address>, right: &Vec<Address>) -> bool {
    if left.len() != right.len() {
        return false;
    }

    for i in 0..left.len() {
        if left.get(i).unwrap() != right.get(i).unwrap() {
            return false;
        }
    }

    true
}

fn reverse_order(env: &Env, members: &Vec<Address>) -> Vec<Address> {
    let mut reversed = Vec::new(env);
    let mut index = members.len();

    while index > 0 {
        index -= 1;
        reversed.push_back(members.get(index).unwrap());
    }

    reversed
}
