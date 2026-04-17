use soroban_sdk::{testutils::Address as _, token::StellarAssetClient, Address, Env, String};

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
#[should_panic(expected = "Unauthorized")]
fn test_admin_cannot_leave_own_group() {
    let (env, admin, client, token) = setup_env();
    let group_id = create_test_group(&env, &client, &admin, &token);
    
    // Admin tries to leave their own group (should fail)
    client.leave_group(&admin, &group_id);
}

#[test]
#[should_panic(expected = "GroupNotForming")]
fn test_leave_during_active_state_fails() {
    let (env, admin, client, token) = setup_env();
    let group_id = create_test_group(&env, &client, &admin, &token);
    
    let member1 = Address::generate(&env);
    client.join_group(&member1, &group_id);
    
    // Start the group (moves to Active state)
    client.start_group(&admin, &group_id);
    
    // Try to leave during Active state (should fail)
    client.leave_group(&member1, &group_id);
}

#[test]
#[should_panic(expected = "NotMember")]
fn test_non_member_leaving_fails() {
    let (env, admin, client, token) = setup_env();
    let group_id = create_test_group(&env, &client, &admin, &token);
    
    // Non-member tries to leave (should fail)
    let non_member = Address::generate(&env);
    client.leave_group(&non_member, &group_id);
}

#[test]
fn test_leave_updates_member_groups_index() {
    let (env, admin, client, token) = setup_env();
    let group_id = create_test_group(&env, &client, &admin, &token);
    
    let member1 = Address::generate(&env);
    client.join_group(&member1, &group_id);
    
    // Verify member1 is in the group
    let member_groups = client.get_member_groups(&member1);
    assert_eq!(member_groups.len(), 1);
    assert_eq!(member_groups.get(0).unwrap(), group_id);
    
    // Member leaves
    client.leave_group(&member1, &group_id);
    
    // Verify member1 is no longer in any groups
    let member_groups = client.get_member_groups(&member1);
    assert_eq!(member_groups.len(), 0);
}

#[test]
fn test_leave_removes_from_members_list() {
    let (env, admin, client, token) = setup_env();
    let group_id = create_test_group(&env, &client, &admin, &token);
    
    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    
    client.join_group(&member1, &group_id);
    client.join_group(&member2, &group_id);
    
    // Verify 3 members (admin + 2)
    let group = client.get_group(&group_id);
    assert_eq!(group.members.len(), 3);
    
    // Member1 leaves
    client.leave_group(&member1, &group_id);
    
    // Verify 2 members remain (admin + member2)
    let group = client.get_group(&group_id);
    assert_eq!(group.members.len(), 2);
    
    // Verify member1 is not in the list
    let mut found_member1 = false;
    for m in group.members.iter() {
        if m == member1 {
            found_member1 = true;
        }
    }
    assert!(!found_member1);
}

#[test]
fn test_multiple_members_leave() {
    let (env, admin, client, token) = setup_env();
    let group_id = create_test_group(&env, &client, &admin, &token);
    
    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    let member3 = Address::generate(&env);
    
    client.join_group(&member1, &group_id);
    client.join_group(&member2, &group_id);
    client.join_group(&member3, &group_id);
    
    // Verify 4 members
    let group = client.get_group(&group_id);
    assert_eq!(group.members.len(), 4);
    
    // All non-admin members leave
    client.leave_group(&member1, &group_id);
    client.leave_group(&member2, &group_id);
    client.leave_group(&member3, &group_id);
    
    // Verify only admin remains
    let group = client.get_group(&group_id);
    assert_eq!(group.members.len(), 1);
    assert_eq!(group.members.get(0).unwrap(), admin);
}
