#![no_main]

mod common;

use libfuzzer_sys::fuzz_target;
use soroban_sdk::String;

fuzz_target!(|data: &[u8]| {
    let harness = common::setup();
    let contribution_amount = (common::read_i64(data, 0) % 5_000_000) as i128;
    let cycle_length = common::read_u64(data, 8);
    let max_members = data.get(16).map(|value| (value % 8) as u32).unwrap_or(0);

    let _ = harness.client.try_create_group(
        &harness.admin,
        &String::from_str(&harness.env, "Fuzz Create"),
        &harness.token,
        &contribution_amount,
        &cycle_length,
        &max_members,
    );
});
