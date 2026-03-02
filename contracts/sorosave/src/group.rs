use soroban_sdk::{Address, Env, Map, String, Vec};

use crate::errors::ContractError;
use crate::storage;
use crate::types::{GroupStatus, GroupTemplate, RoundInfo, SavingsGroup};

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

    // Set payout order to member join order (can be randomized later)
    group.payout_order = group.members.clone();
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

// --- Template Functions ---

const MAX_TEMPLATES_PER_ADMIN: u32 = 10;

pub fn save_template(
    env: &Env,
    admin: Address,
    name: String,
    token: Address,
    contribution_amount: i128,
    cycle_length: u64,
    max_members: u32,
) -> Result<u32, ContractError> {
    admin.require_auth();

    if contribution_amount <= 0 {
        return Err(ContractError::InvalidAmount);
    }
    if max_members < 2 {
        return Err(ContractError::InsufficientMembers);
    }

    let current_count = storage::get_template_counter(env, &admin);
    if current_count >= MAX_TEMPLATES_PER_ADMIN {
        return Err(ContractError::TemplateLimitReached);
    }

    let template_id = current_count + 1;
    let template = GroupTemplate {
        id: template_id,
        name,
        token,
        contribution_amount,
        cycle_length,
        max_members,
    };

    storage::set_template(env, &admin, &template);
    storage::set_template_counter(env, &admin, template_id);

    env.events()
        .publish((crate::symbol_short!("tmpl_save"),), (admin, template_id));

    Ok(template_id)
}

pub fn get_template(
    env: &Env,
    admin: Address,
    template_id: u32,
) -> Result<GroupTemplate, ContractError> {
    storage::get_template(env, &admin, template_id)
        .ok_or(ContractError::TemplateNotFound)
}

pub fn create_from_template(
    env: &Env,
    admin: Address,
    template_id: u32,
    name: String,
) -> Result<u64, ContractError> {
    admin.require_auth();

    let template = storage::get_template(env, &admin, template_id)
        .ok_or(ContractError::TemplateNotFound)?;

    create_group(
        env,
        admin,
        name,
        template.token,
        template.contribution_amount,
        template.cycle_length,
        template.max_members,
    )
}
