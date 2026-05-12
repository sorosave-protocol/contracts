#![no_main]

use libfuzzer_sys::fuzz_target;
use soroban_sdk::{
    testutils::Address as _,
    token::{Client as TokenClient, StellarAssetClient},
    Address, Env, String,
};
use sorosave::{SoroSaveContract, SoroSaveContractClient};

fn byte(data: &[u8], offset: &mut usize) -> u8 {
    let value = data.get(*offset).copied().unwrap_or(0);
    *offset = offset.saturating_add(1);
    value
}

fuzz_target!(|data: &[u8]| {
    let env = Env::default();
    env.mock_all_auths();

    let mut offset = 0usize;
    let member_count = u32::from(byte(data, &mut offset) % 6) + 2;
    let contribution_amount = i128::from(byte(data, &mut offset) % 200) + 1;
    let extra_action = byte(data, &mut offset) % 4;

    let admin = Address::generate(&env);
    let contract_id = env.register(SoroSaveContract, (&admin,));
    let client = SoroSaveContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(token_admin);
    let token = token_id.address();
    let token_admin_client = StellarAssetClient::new(&env, &token);
    let token_client = TokenClient::new(&env, &token);

    let mut members = soroban_sdk::Vec::new(&env);
    members.push_back(admin.clone());
    for _ in 1..member_count {
        members.push_back(Address::generate(&env));
    }

    for member in members.iter() {
        token_admin_client.mint(&member, &(contribution_amount * i128::from(member_count + 1)));
    }

    let group_id = client.create_group(
        &admin,
        &String::from_str(&env, "contribution fuzz"),
        &token,
        &contribution_amount,
        &100,
        &member_count,
    );

    for index in 1..member_count {
        let member = members.get(index).unwrap();
        client.join_group(&member, &group_id);
    }
    client.start_group(&admin, &group_id);

    for member in members.iter() {
        let result = client.try_contribute(&member, &group_id);
        assert!(matches!(result, Ok(Ok(()))));
    }

    let round = client.get_round_status(&group_id, &1);
    assert!(round.is_complete);
    assert_eq!(round.total_contributed, contribution_amount * i128::from(member_count));
    assert_eq!(token_client.balance(&contract_id), round.total_contributed);

    match extra_action {
        0 => {
            let duplicate = members.get(0).unwrap();
            assert!(!matches!(
                client.try_contribute(&duplicate, &group_id),
                Ok(Ok(()))
            ));
        }
        1 => {
            let outsider = Address::generate(&env);
            token_admin_client.mint(&outsider, &contribution_amount);
            assert!(!matches!(
                client.try_contribute(&outsider, &group_id),
                Ok(Ok(()))
            ));
        }
        _ => {}
    }
});
