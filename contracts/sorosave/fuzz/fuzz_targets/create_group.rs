#![no_main]

use libfuzzer_sys::fuzz_target;
use soroban_sdk::{testutils::Address as _, Address, Env, String};
use sorosave::{SoroSaveContract, SoroSaveContractClient};

struct FuzzData<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> FuzzData<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn byte(&mut self) -> u8 {
        let value = self.bytes.get(self.offset).copied().unwrap_or(0);
        self.offset = self.offset.saturating_add(1);
        value
    }

    fn u32(&mut self) -> u32 {
        let mut buf = [0u8; 4];
        for item in &mut buf {
            *item = self.byte();
        }
        u32::from_le_bytes(buf)
    }

    fn u64(&mut self) -> u64 {
        let mut buf = [0u8; 8];
        for item in &mut buf {
            *item = self.byte();
        }
        u64::from_le_bytes(buf)
    }
}

fuzz_target!(|data: &[u8]| {
    let env = Env::default();
    env.mock_all_auths();

    let mut input = FuzzData::new(data);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let contract_id = env.register(SoroSaveContract, (&admin,));
    let client = SoroSaveContractClient::new(&env, &contract_id);

    let raw_amount = i128::from(input.u64() % 1_000_000);
    let contribution_amount = match input.byte() % 5 {
        0 => 0,
        1 => -raw_amount,
        2 => 1,
        3 => i128::MAX,
        _ => raw_amount,
    };
    let cycle_length = match input.byte() % 4 {
        0 => 0,
        1 => 1,
        2 => u64::MAX,
        _ => input.u64(),
    };
    let max_members = match input.byte() % 5 {
        0 => 0,
        1 => 1,
        2 => 2,
        3 => 20,
        _ => (input.u32() % 64) + 2,
    };

    let result = client.try_create_group(
        &admin,
        &String::from_str(&env, "fuzz group"),
        &token,
        &contribution_amount,
        &cycle_length,
        &max_members,
    );

    if contribution_amount > 0 && max_members >= 2 {
        let group_id = match result {
            Ok(Ok(group_id)) => group_id,
            other => panic!("valid fuzz group should be created: {other:?}"),
        };
        let group = client.get_group(&group_id);
        assert_eq!(group.contribution_amount, contribution_amount);
        assert_eq!(group.max_members, max_members);
        assert_eq!(group.members.len(), 1);
    } else {
        assert!(!matches!(result, Ok(Ok(_))));
    }
});
