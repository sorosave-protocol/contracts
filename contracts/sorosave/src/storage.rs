use soroban_sdk::{Address, Env, Vec};

use crate::types::{DataKey, Dispute, MemberReputation, RoundInfo, SavingsGroup};

const INSTANCE_TTL_THRESHOLD: u32 = 100;
const INSTANCE_TTL_EXTEND: u32 = 500;
const PERSISTENT_TTL_THRESHOLD: u32 = 100;
const PERSISTENT_TTL_EXTEND: u32 = 1000;

// --- Admin ---

pub fn get_admin(env: &Env) -> Address {
    env.storage().instance().get(&DataKey::Admin).unwrap()
}

pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::Admin, admin);
    extend_instance_ttl(env);
}

pub fn has_admin(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::Admin)
}

// --- Group Counter ---

pub fn get_group_counter(env: &Env) -> u64 {
    env.storage()
        .instance()
        .get(&DataKey::GroupCounter)
        .unwrap_or(0)
}

pub fn set_group_counter(env: &Env, counter: u64) {
    env.storage()
        .instance()
        .set(&DataKey::GroupCounter, &counter);
    extend_instance_ttl(env);
}

// --- Group ---

pub fn get_group(env: &Env, group_id: u64) -> Option<SavingsGroup> {
    let key = DataKey::Group(group_id);
    let result = env.storage().persistent().get(&key);
    if result.is_some() {
        extend_persistent_ttl(env, &key);
    }
    result
}

pub fn set_group(env: &Env, group: &SavingsGroup) {
    let key = DataKey::Group(group.id);
    env.storage().persistent().set(&key, group);
    extend_persistent_ttl(env, &key);
}

// --- Round ---

pub fn get_round(env: &Env, group_id: u64, round: u32) -> Option<RoundInfo> {
    let key = DataKey::Round(group_id, round);
    let result = env.storage().persistent().get(&key);
    if result.is_some() {
        extend_persistent_ttl(env, &key);
    }
    result
}

pub fn set_round(env: &Env, group_id: u64, round_info: &RoundInfo) {
    let key = DataKey::Round(group_id, round_info.round_number);
    env.storage().persistent().set(&key, round_info);
    extend_persistent_ttl(env, &key);
}

// --- Member Groups ---

pub fn get_member_groups(env: &Env, member: &Address) -> Vec<u64> {
    let key = DataKey::MemberGroups(member.clone());
    env.storage()
        .persistent()
        .get(&key)
        .unwrap_or(Vec::new(env))
}

pub fn add_member_group(env: &Env, member: &Address, group_id: u64) {
    let key = DataKey::MemberGroups(member.clone());
    let mut groups = get_member_groups(env, member);
    groups.push_back(group_id);
    env.storage().persistent().set(&key, &groups);
    extend_persistent_ttl(env, &key);
}

pub fn remove_member_group(env: &Env, member: &Address, group_id: u64) {
    let key = DataKey::MemberGroups(member.clone());
    let groups = get_member_groups(env, member);
    let mut new_groups = Vec::new(env);
    for g in groups.iter() {
        if g != group_id {
            new_groups.push_back(g);
        }
    }
    env.storage().persistent().set(&key, &new_groups);
    extend_persistent_ttl(env, &key);
}

// --- Member Reputation ---

pub fn get_member_reputation(env: &Env, member: &Address) -> MemberReputation {
    let key = DataKey::MemberReputation(member.clone());
    let result = env.storage().persistent().get(&key);
    if let Some(reputation) = result {
        extend_persistent_ttl(env, &key);
        reputation
    } else {
        MemberReputation {
            groups_completed: 0,
            on_time_contributions: 0,
            defaults: 0,
        }
    }
}

pub fn set_member_reputation(env: &Env, member: &Address, reputation: &MemberReputation) {
    let key = DataKey::MemberReputation(member.clone());
    env.storage().persistent().set(&key, reputation);
    extend_persistent_ttl(env, &key);
}

pub fn increment_on_time_contributions(env: &Env, member: &Address) {
    let mut reputation = get_member_reputation(env, member);
    reputation.on_time_contributions += 1;
    set_member_reputation(env, member, &reputation);
}

pub fn increment_groups_completed(env: &Env, member: &Address) {
    let mut reputation = get_member_reputation(env, member);
    reputation.groups_completed += 1;
    set_member_reputation(env, member, &reputation);
}

// --- Dispute ---

#[allow(dead_code)]
pub fn get_dispute(env: &Env, group_id: u64) -> Option<Dispute> {
    let key = DataKey::Dispute(group_id);
    env.storage().persistent().get(&key)
}

pub fn set_dispute(env: &Env, group_id: u64, dispute: &Dispute) {
    let key = DataKey::Dispute(group_id);
    env.storage().persistent().set(&key, dispute);
    extend_persistent_ttl(env, &key);
}

pub fn remove_dispute(env: &Env, group_id: u64) {
    let key = DataKey::Dispute(group_id);
    env.storage().persistent().remove(&key);
}

// --- TTL Management ---

fn extend_instance_ttl(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND);
}

fn extend_persistent_ttl(env: &Env, key: &DataKey) {
    env.storage()
        .persistent()
        .extend_ttl(key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND);
}
