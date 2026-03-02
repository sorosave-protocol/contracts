#[test]
fn test_emergency_withdrawal_unequal_contributions() {
    // Setup environment
    let (env, admin, client, token) = setup_env();
    
    // Create group with 5 members, 5 rounds
    let group_id = client.create_group(
        &admin,
        &String::from_str(&env, "Emergency Test Group"),
        &token,
        &1_000_000, // 1 token contribution
        &86400,     // 1 day cycle
        &5,         // 5 members
    );
    
    // Add 4 more members (total 5 including admin)
    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    let member3 = Address::generate(&env);
    let member4 = Address::generate(&env);
    
    client.join_group(&member1, &group_id);
    client.join_group(&member2, &group_id);
    client.join_group(&member3, &group_id);
    client.join_group(&member4, &group_id);
    
    // Start the group
    client.start_group(&admin, &group_id);
    
    // Get initial balances
    let admin_balance_before = get_balance(&env, &token, &admin);
    let m1_balance_before = get_balance(&env, &token, &member1);
    let m2_balance_before = get_balance(&env, &token, &member2);
    let m3_balance_before = get_balance(&env, &token, &member3);
    let m4_balance_before = get_balance(&env, &token, &member4);
    
    // Round 1: All members contribute
    client.contribute(&admin, &group_id);
    client.contribute(&member1, &group_id);
    client.contribute(&member2, &group_id);
    client.contribute(&member3, &group_id);
    client.contribute(&member4, &group_id);
    
    // Advance to round 2
    env.ledger().set_timestamp(env.ledger().timestamp() + 86401);
    
    // Round 2: Only 3 members contribute (unequal contributions)
    client.contribute(&admin, &group_id);
    client.contribute(&member1, &group_id);
    client.contribute(&member2, &group_id);
    // member3 and member4 skip round 2
    
    // Verify round 2 state
    let round2 = client.get_round_status(&group_id, &2);
    assert_eq!(round2.contributions.len(), 3);
    
    // Advance to round 3
    env.ledger().set_timestamp(env.ledger().timestamp() + 86401);
    
    // Trigger emergency withdrawal mid-round 3
    // At this point:
    // - admin: contributed round 1, round 2 (2 contributions)
    // - member1: contributed round 1, round 2 (2 contributions)
    // - member2: contributed round 1, round 2 (2 contributions)
    // - member3: contributed round 1 only (1 contribution)
    // - member4: contributed round 1 only (1 contribution)
    
    client.emergency_withdraw(&admin, &group_id);
    
    // Verify group status
    let group = client.get_group(&group_id);
    assert_eq!(group.status, GroupStatus::Completed);
    
    // Verify proportional distribution
    // Total contributions: 5 (round 1) + 3 (round 2) = 8 contributions = 8,000,000 tokens
    // Distribution should be proportional to actual contributions
    
    let admin_balance_after = get_balance(&env, &token, &admin);
    let m1_balance_after = get_balance(&env, &token, &member1);
    let m2_balance_after = get_balance(&env, &token, &member2);
    let m3_balance_after = get_balance(&env, &token, &member3);
    let m4_balance_after = get_balance(&env, &token, &member4);
    
    // Admin, member1, member2 each contributed 2x = should get 2/8 = 25% each
    // member3, member4 each contributed 1x = should get 1/8 = 12.5% each
    
    let admin_refund = admin_balance_after - admin_balance_before;
    let m1_refund = m1_balance_after - m1_balance_before;
    let m2_refund = m2_balance_after - m2_balance_before;
    let m3_refund = m3_balance_after - m3_balance_before;
    let m4_refund = m4_balance_after - m4_balance_before;
    
    // Verify proportional refunds (with small tolerance for rounding)
    let expected_admin_refund = 2_000_000; // 25% of 8,000,000
    let expected_member_refund = 2_000_000; // 25% for m1, m2
    let expected_partial_refund = 1_000_000; // 12.5% for m3, m4
    
    assert!(
        admin_refund >= expected_admin_refund - 1000 && admin_refund <= expected_admin_refund + 1000,
        "Admin refund should be ~2,000,000, got {}",
        admin_refund
    );
    assert!(
        m1_refund >= expected_member_refund - 1000 && m1_refund <= expected_member_refund + 1000,
        "Member1 refund should be ~2,000,000, got {}",
        m1_refund
    );
    assert!(
        m2_refund >= expected_member_refund - 1000 && m2_refund <= expected_member_refund + 1000,
        "Member2 refund should be ~2,000,000, got {}",
        m2_refund
    );
    assert!(
        m3_refund >= expected_partial_refund - 1000 && m3_refund <= expected_partial_refund + 1000,
        "Member3 refund should be ~1,000,000, got {}",
        m3_refund
    );
    assert!(
        m4_refund >= expected_partial_refund - 1000 && m4_refund <= expected_partial_refund + 1000,
        "Member4 refund should be ~1,000,000, got {}",
        m4_refund
    );
    
    // Verify all funds are returned (no tokens stuck in contract)
    let total_refund = admin_refund + m1_refund + m2_refund + m3_refund + m4_refund;
    let total_contributed = 8_000_000i128; // 8 contributions * 1,000,000
    
    assert!(
        total_refund >= total_contributed - 5000 && total_refund <= total_contributed + 5000,
        "Total refund should equal total contributed (~8,000,000), got {}",
        total_refund
    );
}

// Helper function to get token balance
fn get_balance(env: &Env, token: &Address, account: &Address) &gt; i128 {
    use soroban_sdk::token::TokenClient;
    let token_client = TokenClient::new(env, token);
    token_client.balance(account)
}
