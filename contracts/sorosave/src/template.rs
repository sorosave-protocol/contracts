use soroban_sdk::{Address, Env, String, Vec};

use crate::errors::ContractError;
use crate::storage;
use crate::types::GroupTemplate;

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
        return Err(ContractError::InvalidAmount); // Reuse error, could add new one
    }

    let template = GroupTemplate {
        name,
        token,
        contribution_amount,
        cycle_length,
        max_members,
    };

    let new_index = current_count;
    storage::set_template(env, &admin, new_index, &template);
    storage::set_template_counter(env, &admin, current_count + 1);

    env.events()
        .publish((crate::symbol_short!("tmpl_save"),), (admin, new_index));

    Ok(new_index)
}

pub fn get_template(
    env: &Env,
    admin: Address,
    index: u32,
) -> Result<GroupTemplate, ContractError> {
    storage::get_template(env, &admin, index).ok_or(ContractError::GroupNotFound)
}

pub fn get_admin_templates(env: &Env, admin: Address) -> Vec<GroupTemplate> {
    let count = storage::get_template_counter(env, &admin);
    let mut templates = Vec::new(env);
    
    for i in 0..count {
        if let Some(template) = storage::get_template(env, &admin, i) {
            templates.push_back(template);
        }
    }
    
    templates
}

pub fn create_from_template(
    env: &Env,
    admin: Address,
    template_index: u32,
    name: Option<String>,
) -> Result<u64, ContractError> {
    admin.require_auth();

    let template = storage::get_template(env, &admin, template_index)
        .ok_or(ContractError::GroupNotFound)?;

    // Use provided name or fall back to template name
    let group_name = name.unwrap_or(template.name);

    // Import create_group from group module
    crate::group::create_group(
        env,
        admin,
        group_name,
        template.token,
        template.contribution_amount,
        template.cycle_length,
        template.max_members,
    )
}
