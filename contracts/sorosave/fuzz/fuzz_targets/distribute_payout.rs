#![no_main]

mod common;

use libfuzzer_sys::fuzz_target;
use soroban_sdk::{Address, String};

fuzz_target!(|data: &[u8]| {
    let harness = common::setup();
    let contribution_amount = common::positive_amount(data, 0);
    let member_count = common::member_count(data, 8);
    let members = common::mint_members(
        &harness.env,
        &harness.token,
        member_count - 1,
        contribution_amount * 4,
    );

    let group_id = harness.client.create_group(
        &harness.admin,
        &String::from_str(&harness.env, "Fuzz Payout"),
        &harness.token,
        &contribution_amount,
        &common::cycle_length(data, 9),
        &(member_count as u32),
    );

    let mut actors = std::vec::Vec::<Address>::with_capacity(member_count);
    actors.push(harness.admin.clone());
    for member in members {
        harness.client.join_group(&member, &group_id);
        actors.push(member);
    }

    harness.client.start_group(&harness.admin, &group_id);

    for (index, actor) in actors.iter().enumerate() {
        let should_contribute = data
            .get(17 + index)
            .map(|byte| byte & 1 == 1)
            .unwrap_or(true);
        if should_contribute {
            let _ = harness.client.try_contribute(actor, &group_id);
        }
    }

    for byte in data.iter().skip(17 + actors.len()).take(16) {
        if byte & 1 == 0 {
            let _ = harness.client.try_distribute_payout(&group_id);
        } else {
            let actor = &actors[*byte as usize % actors.len()];
            let _ = harness.client.try_contribute(actor, &group_id);
        }
    }
});
