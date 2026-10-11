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
        &String::from_str(&harness.env, "Fuzz Contribute"),
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

    for byte in data.iter().skip(17).take(64) {
        let actor = &actors[*byte as usize % actors.len()];
        let _ = harness.client.try_contribute(actor, &group_id);
    }
});
