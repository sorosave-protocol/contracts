use soroban_sdk::{Address, Env, String, Vec};

use crate::errors::ContractError;
use crate::storage;
use crate::types::{Dispute, GroupStatus, AdminProposal, ProposalType, ProposalStatus};

pub fn add_admin(env: &Env, current_admin: Address, new_admin: Address) -> Result<(), ContractError> {
    current_admin.require_auth();
    
    let proposal_id = storage::get_next_proposal_id(env);
    let proposal = AdminProposal {
        id: proposal_id,
        proposal_type: ProposalType::AddAdmin,
        target: new_admin.clone(),
        threshold: None,
        proposer: current_admin.clone(),
        approvals: Vec::from_array(env, [current_admin]),
        status: ProposalStatus::Pending,
        created_at: env.ledger().timestamp(),
    };
    
    storage::set_admin_proposal(env, &proposal);
    
    let admins = storage::get_admins(env);
    let threshold = storage::get_admin_threshold(env);
    
    if proposal.approvals.len() >= threshold {
        execute_add_admin_proposal(env, proposal_id)?;
    }
    
    env.events()
        .publish((crate::symbol_short!("add_admn"),), (proposal_id, new_admin));
    
    Ok(())
}

pub fn remove_admin(env: &Env, current_admin: Address, target_admin: Address) -> Result<(), ContractError> {
    current_admin.require_auth();
    
    let admins = storage::get_admins(env);
    if admins.len() <= 1 {
        return Err(ContractError::Unauthorized);
    }
    
    let proposal_id = storage::get_next_proposal_id(env);
    let proposal = AdminProposal {
        id: proposal_id,
        proposal_type: ProposalType::RemoveAdmin,
        target: target_admin.clone(),
        threshold: None,
        proposer: current_admin.clone(),
        approvals: Vec::from_array(env, [current_admin]),
        status: ProposalStatus::Pending,
        created_at: env.ledger().timestamp(),
    };
    
    storage::set_admin_proposal(env, &proposal);
    
    let threshold = storage::get_admin_threshold(env);
    
    if proposal.approvals.len() >= threshold {
        execute_remove_admin_proposal(env, proposal_id)?;
    }
    
    env.events()
        .publish((crate::symbol_short!("rem_admn"),), (proposal_id, target_admin));
    
    Ok(())
}

pub fn set_threshold(env: &Env, admin: Address, new_threshold: u32) -> Result<(), ContractError> {
    admin.require_auth();
    
    let admins = storage::get_admins(env);
    if new_threshold == 0 || new_threshold > admins.len() {
        return Err(ContractError::Unauthorized);
    }
    
    let proposal_id = storage::get_next_proposal_id(env);
    let proposal = AdminProposal {
        id: proposal_id,
        proposal_type: ProposalType::SetThreshold,
        target: admin.clone(),
        threshold: Some(new_threshold),
        proposer: admin.clone(),
        approvals: Vec::from_array(env, [admin]),
        status: ProposalStatus::Pending,
        created_at: env.ledger().timestamp(),
    };
    
    storage::set_admin_proposal(env, &proposal);
    
    let current_threshold = storage::get_admin_threshold(env);
    
    if proposal.approvals.len() >= current_threshold {
        execute_set_threshold_proposal(env, proposal_id)?;
    }
    
    env.events()
        .publish((crate::symbol_short!("set_thrs"),), (proposal_id, new_threshold));
    
    Ok(())
}

pub fn approve_admin_proposal(env: &Env, admin: Address, proposal_id: u64) -> Result<(), ContractError> {
    admin.require_auth();
    
    let admins = storage::get_admins(env);
    if !admins.contains(&admin) {
        return Err(ContractError::Unauthorized);
    }
    
    let mut proposal = storage::get_admin_proposal(env, proposal_id)
        .ok_or(ContractError::ProposalNotFound)?;
    
    if proposal.status != ProposalStatus::Pending {
        return Err(ContractError::ProposalNotActive);
    }
    
    if proposal.approvals.contains(&admin) {
        return Err(ContractError::AlreadyApproved);
    }
    
    proposal.approvals.push_back(admin.clone());
    storage::set_admin_proposal(env, &proposal);
    
    let threshold = storage::get_admin_threshold(env);
    
    if proposal.approvals.len() >= threshold {
        match proposal.proposal_type {
            ProposalType::AddAdmin => execute_add_admin_proposal(env, proposal_id)?,
            ProposalType::RemoveAdmin => execute_remove_admin_proposal(env, proposal_id)?,
            ProposalType::SetThreshold => execute_set_threshold_proposal(env, proposal_id)?,
        }
    }
    
    env.events()
        .publish((crate::symbol_short!("appr_prop"),), (proposal_id, admin));
    
    Ok(())
}

fn execute_add_admin_proposal(env: &Env, proposal_id: u64) -> Result<(), ContractError> {
    let mut proposal = storage::get_admin_proposal(env, proposal_id)
        .ok_or(ContractError::ProposalNotFound)?;
    
    let mut admins = storage::get_admins(env);
    admins.push_back(proposal.target.clone());
    storage::set_admins(env, &admins);
    
    proposal.status = ProposalStatus::Executed;
    storage::set_admin_proposal(env, &proposal);
    
    env.events()
        .publish((crate::symbol_short!("exec_add"),), (proposal_id, proposal.target.clone()));
    
    Ok(())
}

fn execute_remove_admin_proposal(env: &Env, proposal_id: u64) -> Result<(), ContractError> {
    let mut proposal = storage::get_admin_proposal(env, proposal_id)
        .ok_or(ContractError::ProposalNotFound)?;
    
    let mut admins = storage::get_admins(env);
    if let Some(index) = admins.iter().position(|admin| admin == proposal.target) {
        admins.remove(index as u32);
        storage::set_admins(env, &admins);
    }
    
    proposal.status = ProposalStatus::Executed;
    storage::set_admin_proposal(env, &proposal);
    
    env.events()
        .publish((crate::symbol_short!("exec_rem"),), (proposal_id, proposal.target.clone()));
    
    Ok(())
}

fn execute_set_threshold_proposal(env: &Env, proposal_id: u64) -> Result<(), ContractError> {
    let mut proposal = storage::get_admin_proposal(env, proposal_id)
        .ok_or(ContractError::ProposalNotFound)?;
    
    if let Some(new_threshold) = proposal.threshold {
        storage::set_admin_threshold(env, new_threshold);
    }
    
    proposal.status = ProposalStatus::Executed;
    storage::set_admin_proposal(env, &proposal);
    
    env.events()
        .publish((crate::symbol_short!("exec_thr"),), (proposal_id, proposal.threshold.unwrap_or(0)));
    
    Ok(())
}

pub fn pause_group(env: &Env, admin: Address, group_id: u64) -> Result<(), ContractError> {
    admin.require_auth();

    let admins = storage::get_admins(env);
    if !admins.contains(&admin) {
        return Err(ContractError::Unauthorized);
    }

    let mut group = storage::get_group(env, group_id).ok_or(ContractError::GroupNotFound)?;

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

    let admins = storage::get_admins(env);
    if !admins.contains(&admin) {
        return Err(ContractError::Unauthorized);
    }

    let mut group = storage::get_group(env, group_id).ok_or(ContractError::GroupNotFound)?;

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

    let group = storage::get_group(env, group_id).ok_or(ContractError::GroupNotFound)?;
    
    if !group.members.contains(&member) {
        return Err(ContractError::NotGroupMember);
    }

    if group.status != GroupStatus::Active {
        return Err(ContractError::GroupNotActive);
    }

    let dispute_id = storage::get_next_dispute_id(env);
    let dispute = Dispute {
        id: dispute_id,
        group_id,
        member: member.clone(),
        reason: reason.clone(),
        resolved: false,
        created_at: env.ledger().timestamp(),
    };

    storage::set_dispute(env, &dispute);

    env.events()
        .publish((crate::symbol_short!("dispute"),), (dispute_id, group_id, member));

    Ok(())
}