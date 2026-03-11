extern crate std;

use soroban_sdk::{testutils::Address as _, token::StellarAssetClient, Address, Env, String};
use self::std::panic::{catch_unwind, AssertUnwindSafe};

use crate::types::GroupStatus;
use crate::{SoroSaveContract, SoroSaveContractClient};

fn setup_env() -> (Env, Address, SoroSaveContractClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(SoroSaveContract, (&admin,));
    let client = SoroSaveContractClient::new(&env, &contract_id);

    // Create a test token
    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_client = StellarAssetClient::new(&env, &token_id.address());

    // Mint tokens to admin
    token_client.mint(&admin, &10_000_000);

    (env, admin, client, token_id.address())
}

fn create_test_group(
    env: &Env,
    client: &SoroSaveContractClient,
    admin: &Address,
    token: &Address,
) -> u64 {
    client.create_group(
        admin,
        &String::from_str(env, "Test Savings Group"),
        token,
        &1_000_000, // 1 token (7 decimals)
        &86400,     // 1 day cycle
        &5,         // max 5 members
    )
}

fn assert_panics<F, R>(f: F)
where
    F: FnOnce() -> R,
{
    assert!(catch_unwind(AssertUnwindSafe(f)).is_err());
}

#[test]
fn test_create_group() {
    let (env, admin, client, token) = setup_env();
    let group_id = create_test_group(&env, &client, &admin, &token);
    assert_eq!(group_id, 1);

    let group = client.get_group(&group_id);
    assert_eq!(group.admin, admin);
    assert_eq!(group.contribution_amount, 1_000_000);
    assert_eq!(group.max_members, 5);
    assert_eq!(group.status, GroupStatus::Forming);
    assert_eq!(group.members.len(), 1);
}

#[test]
fn test_join_group() {
    let (env, admin, client, token) = setup_env();
    let group_id = create_test_group(&env, &client, &admin, &token);

    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);

    client.join_group(&member1, &group_id);
    client.join_group(&member2, &group_id);

    let group = client.get_group(&group_id);
    assert_eq!(group.members.len(), 3); // admin + 2 members
}

#[test]
fn test_leave_group() {
    let (env, admin, client, token) = setup_env();
    let group_id = create_test_group(&env, &client, &admin, &token);

    let member1 = Address::generate(&env);
    client.join_group(&member1, &group_id);

    assert_eq!(client.get_group(&group_id).members.len(), 2);

    client.leave_group(&member1, &group_id);
    assert_eq!(client.get_group(&group_id).members.len(), 1);
}

#[test]
fn test_start_group() {
    let (env, admin, client, token) = setup_env();
    let group_id = create_test_group(&env, &client, &admin, &token);

    let member1 = Address::generate(&env);
    client.join_group(&member1, &group_id);

    client.start_group(&admin, &group_id);

    let group = client.get_group(&group_id);
    assert_eq!(group.status, GroupStatus::Active);
    assert_eq!(group.current_round, 1);
    assert_eq!(group.total_rounds, 2);
    assert_eq!(group.payout_order.len(), 2);
}

#[test]
fn test_full_cycle() {
    let (env, admin, client, token) = setup_env();
    let group_id = create_test_group(&env, &client, &admin, &token);

    let member1 = Address::generate(&env);
    client.join_group(&member1, &group_id);

    // Mint tokens to member
    let _token_admin_client = StellarAssetClient::new(&env, &token);
    let mint_admin = Address::generate(&env);
    // Re-register to get a token we can mint from
    let token_id = env.register_stellar_asset_contract_v2(mint_admin.clone());
    let token_sac = StellarAssetClient::new(&env, &token_id.address());
    token_sac.mint(&admin, &10_000_000);
    token_sac.mint(&member1, &10_000_000);

    // Create a new group with the token we control
    let group_id = client.create_group(
        &admin,
        &String::from_str(&env, "Full Cycle Test"),
        &token_id.address(),
        &1_000_000,
        &86400,
        &5,
    );
    client.join_group(&member1, &group_id);
    client.start_group(&admin, &group_id);

    // Round 1: both contribute
    client.contribute(&admin, &group_id);
    client.contribute(&member1, &group_id);

    let round = client.get_round_status(&group_id, &1);
    assert!(round.is_complete);

    // Distribute round 1 payout
    client.distribute_payout(&group_id);

    // Round 2: both contribute
    let group = client.get_group(&group_id);
    assert_eq!(group.current_round, 2);

    client.contribute(&admin, &group_id);
    client.contribute(&member1, &group_id);

    client.distribute_payout(&group_id);

    // Group should be completed
    let group = client.get_group(&group_id);
    assert_eq!(group.status, GroupStatus::Completed);
}

#[test]
fn test_member_groups() {
    let (env, admin, client, token) = setup_env();

    let group1 = create_test_group(&env, &client, &admin, &token);
    let group2 = client.create_group(
        &admin,
        &String::from_str(&env, "Second Group"),
        &token,
        &500_000,
        &43200,
        &3,
    );

    let groups = client.get_member_groups(&admin);
    assert_eq!(groups.len(), 2);
    assert_eq!(groups.get(0).unwrap(), group1);
    assert_eq!(groups.get(1).unwrap(), group2);
}

#[test]
fn test_pause_resume_group() {
    let (env, admin, client, token) = setup_env();
    let group_id = create_test_group(&env, &client, &admin, &token);

    let member1 = Address::generate(&env);
    client.join_group(&member1, &group_id);
    client.start_group(&admin, &group_id);

    // Pause
    client.pause_group(&admin, &group_id);
    assert_eq!(client.get_group(&group_id).status, GroupStatus::Paused);

    // Resume
    client.resume_group(&admin, &group_id);
    assert_eq!(client.get_group(&group_id).status, GroupStatus::Active);
}

#[test]
fn test_dispute_flow() {
    let (env, admin, client, token) = setup_env();
    let group_id = create_test_group(&env, &client, &admin, &token);

    let member1 = Address::generate(&env);
    client.join_group(&member1, &group_id);
    client.start_group(&admin, &group_id);

    // Member raises dispute
    client.raise_dispute(
        &member1,
        &group_id,
        &String::from_str(&env, "Suspicious activity"),
    );
    assert_eq!(client.get_group(&group_id).status, GroupStatus::Disputed);

    // Admin resolves
    client.resolve_dispute(&admin, &group_id);
    assert_eq!(client.get_group(&group_id).status, GroupStatus::Active);
}

#[test]
fn test_set_group_admin() {
    let (env, admin, client, token) = setup_env();
    let group_id = create_test_group(&env, &client, &admin, &token);

    let new_admin = Address::generate(&env);
    client.set_group_admin(&admin, &group_id, &new_admin);

    let group = client.get_group(&group_id);
    assert_eq!(group.admin, new_admin);
}

#[test]
fn test_only_protocol_admin_can_toggle_protocol_pause() {
    let (env, admin, client, _token) = setup_env();
    let outsider = Address::generate(&env);

    assert_panics(|| client.pause_protocol(&outsider));

    client.pause_protocol(&admin);
    assert!(client.is_protocol_paused());

    assert_panics(|| client.unpause_protocol(&outsider));

    client.unpause_protocol(&admin);
    assert!(!client.is_protocol_paused());
}

#[test]
fn test_protocol_pause_blocks_state_changes() {
    let (env, admin, client, token) = setup_env();
    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    let replacement_admin = Address::generate(&env);

    let forming_group_id = create_test_group(&env, &client, &admin, &token);
    client.join_group(&member1, &forming_group_id);

    let active_group_id = client.create_group(
        &admin,
        &String::from_str(&env, "Active Group"),
        &token,
        &1_000_000,
        &86400,
        &5,
    );
    client.join_group(&member1, &active_group_id);
    client.start_group(&admin, &active_group_id);

    client.pause_protocol(&admin);
    assert!(client.is_protocol_paused());

    assert_panics(|| {
        client.create_group(
            &admin,
            &String::from_str(&env, "Blocked Group"),
            &token,
            &1_000_000,
            &86400,
            &5,
        )
    });
    assert_panics(|| client.join_group(&member2, &forming_group_id));
    assert_panics(|| client.leave_group(&member1, &forming_group_id));
    assert_panics(|| client.start_group(&admin, &forming_group_id));
    assert_panics(|| client.contribute(&admin, &active_group_id));
    assert_panics(|| client.distribute_payout(&active_group_id));
    assert_panics(|| client.pause_group(&admin, &active_group_id));
    assert_panics(|| client.resume_group(&admin, &active_group_id));
    assert_panics(|| {
        client.raise_dispute(
            &member1,
            &active_group_id,
            &String::from_str(&env, "Protocol pause should block this"),
        )
    });
    assert_panics(|| client.resolve_dispute(&admin, &active_group_id));
    assert_panics(|| client.emergency_withdraw(&admin, &active_group_id));
    assert_panics(|| client.set_group_admin(&admin, &active_group_id, &replacement_admin));
}

#[test]
fn test_protocol_unpause_restores_state_changes() {
    let (env, admin, client, token) = setup_env();
    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);

    let token_client = StellarAssetClient::new(&env, &token);
    token_client.mint(&member1, &5_000_000);
    token_client.mint(&member2, &5_000_000);

    let group_id = create_test_group(&env, &client, &admin, &token);
    client.join_group(&member1, &group_id);

    client.pause_protocol(&admin);
    assert_panics(|| client.join_group(&member2, &group_id));

    client.unpause_protocol(&admin);
    assert!(!client.is_protocol_paused());

    client.join_group(&member2, &group_id);
    client.start_group(&admin, &group_id);
    client.contribute(&admin, &group_id);
    client.contribute(&member1, &group_id);
    client.contribute(&member2, &group_id);

    let round = client.get_round_status(&group_id, &1);
    assert!(round.is_complete);
    assert_eq!(round.total_contributed, 3_000_000);

    client.distribute_payout(&group_id);
    assert_eq!(client.get_group(&group_id).current_round, 2);
}
