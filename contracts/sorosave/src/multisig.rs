use soroban_sdk::{Address, Env, Vec};

use crate::errors::ContractError;
use crate::storage;
use crate::types::{MultiSigAction, MultiSigProposal, SavingsGroup};

pub fn is_group_admin(group: &SavingsGroup, admin: &Address) -> bool {
    if *admin == group.admin {
        return true;
    }

    for existing in group.admins.iter() {
        if existing == *admin {
            return true;
        }
    }

    false
}

pub fn can_single_admin_act(env: &Env, group: &SavingsGroup, admin: &Address) -> bool {
    is_group_admin(group, admin) || *admin == storage::get_admin(env)
}

pub fn ensure_group_admin(group: &SavingsGroup, admin: &Address) -> Result<(), ContractError> {
    if !is_group_admin(group, admin) {
        return Err(ContractError::Unauthorized);
    }

    Ok(())
}

pub fn add_admin(
    env: &Env,
    current_admin: Address,
    group_id: u64,
    new_admin: Address,
) -> Result<(), ContractError> {
    current_admin.require_auth();

    let mut group = storage::get_group(env, group_id).ok_or(ContractError::GroupNotFound)?;
    if !can_single_admin_act(env, &group, &current_admin) {
        return Err(ContractError::Unauthorized);
    }

    for admin in group.admins.iter() {
        if admin == new_admin {
            return Err(ContractError::AlreadyMember);
        }
    }

    group.admins.push_back(new_admin.clone());
    storage::set_group(env, &group);

    env.events()
        .publish((crate::symbol_short!("adm_add"),), (group_id, new_admin));

    Ok(())
}

pub fn remove_admin(
    env: &Env,
    current_admin: Address,
    group_id: u64,
    admin_to_remove: Address,
) -> Result<(), ContractError> {
    current_admin.require_auth();

    let mut group = storage::get_group(env, group_id).ok_or(ContractError::GroupNotFound)?;
    if !can_single_admin_act(env, &group, &current_admin) {
        return Err(ContractError::Unauthorized);
    }
    if admin_to_remove == group.admin {
        return Err(ContractError::Unauthorized);
    }

    let mut found = false;
    let mut admins = Vec::new(env);
    for admin in group.admins.iter() {
        if admin == admin_to_remove {
            found = true;
        } else {
            admins.push_back(admin);
        }
    }

    if !found {
        return Err(ContractError::NotMember);
    }
    if group.admin_threshold > admins.len() {
        return Err(ContractError::InvalidThreshold);
    }
    if group.admin_threshold > 1 && admins.len() < 2 {
        return Err(ContractError::InvalidThreshold);
    }

    group.admins = admins;
    storage::set_group(env, &group);

    env.events().publish(
        (crate::symbol_short!("adm_rem"),),
        (group_id, admin_to_remove),
    );

    Ok(())
}

pub fn set_threshold(
    env: &Env,
    current_admin: Address,
    group_id: u64,
    threshold: u32,
) -> Result<(), ContractError> {
    current_admin.require_auth();

    let mut group = storage::get_group(env, group_id).ok_or(ContractError::GroupNotFound)?;
    if !can_single_admin_act(env, &group, &current_admin) {
        return Err(ContractError::Unauthorized);
    }
    if threshold == 0 || threshold > group.admins.len() {
        return Err(ContractError::InvalidThreshold);
    }
    if threshold > 1 && group.admins.len() < 2 {
        return Err(ContractError::InvalidThreshold);
    }

    group.admin_threshold = threshold;
    storage::set_group(env, &group);

    env.events()
        .publish((crate::symbol_short!("adm_thr"),), (group_id, threshold));

    Ok(())
}

pub fn approve_sensitive_action(
    env: &Env,
    group: &SavingsGroup,
    admin: Address,
    action: MultiSigAction,
) -> Result<bool, ContractError> {
    if group.admin_threshold <= 1 {
        if !can_single_admin_act(env, group, &admin) {
            return Err(ContractError::Unauthorized);
        }
        return Ok(true);
    }

    ensure_group_admin(group, &admin)?;
    if group.admins.len() < 2 || group.admin_threshold > group.admins.len() {
        return Err(ContractError::InvalidThreshold);
    }

    let mut proposal =
        storage::get_multisig_proposal(env, group.id, &action).unwrap_or(MultiSigProposal {
            action: action.clone(),
            approvals: Vec::new(env),
            created_at: env.ledger().timestamp(),
        });

    for approver in proposal.approvals.iter() {
        if approver == admin {
            return Err(ContractError::ProposalAlreadyApproved);
        }
    }

    proposal.approvals.push_back(admin.clone());

    if proposal.approvals.len() >= group.admin_threshold {
        storage::remove_multisig_proposal(env, group.id, &action);
        env.events().publish(
            (crate::symbol_short!("ms_exec"),),
            (group.id, action, admin),
        );
        return Ok(true);
    }

    storage::set_multisig_proposal(env, group.id, &proposal);
    env.events().publish(
        (crate::symbol_short!("ms_appr"),),
        (group.id, action, admin),
    );

    Ok(false)
}
