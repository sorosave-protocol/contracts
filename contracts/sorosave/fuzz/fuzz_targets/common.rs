#![allow(dead_code)]

use soroban_sdk::{testutils::Address as _, token::StellarAssetClient, Address, Env};
use sorosave::{SoroSaveContract, SoroSaveContractClient};

pub struct Harness {
    pub env: Env,
    pub admin: Address,
    pub client: SoroSaveContractClient<'static>,
    pub token: Address,
}

pub fn setup() -> Harness {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(SoroSaveContract, (&admin,));
    let client = SoroSaveContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(token_admin);
    let token = token_id.address();
    let token_client = StellarAssetClient::new(&env, &token);
    token_client.mint(&admin, &1_000_000_000_000);

    Harness {
        env,
        admin,
        client,
        token,
    }
}

pub fn read_i64(data: &[u8], offset: usize) -> i64 {
    let mut bytes = [0_u8; 8];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = data.get(offset + index).copied().unwrap_or_default();
    }
    i64::from_le_bytes(bytes)
}

pub fn read_u64(data: &[u8], offset: usize) -> u64 {
    let mut bytes = [0_u8; 8];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = data.get(offset + index).copied().unwrap_or_default();
    }
    u64::from_le_bytes(bytes)
}

pub fn positive_amount(data: &[u8], offset: usize) -> i128 {
    (read_u64(data, offset) % 5_000_000 + 1) as i128
}

pub fn cycle_length(data: &[u8], offset: usize) -> u64 {
    read_u64(data, offset) % 604_800 + 1
}

pub fn member_count(data: &[u8], offset: usize) -> usize {
    data.get(offset)
        .map(|value| (value % 5 + 2) as usize)
        .unwrap_or(2)
}

pub fn mint_members(
    env: &Env,
    token: &Address,
    count: usize,
    amount: i128,
) -> std::vec::Vec<Address> {
    let token_client = StellarAssetClient::new(env, token);
    let mut members = std::vec::Vec::with_capacity(count);
    for _ in 0..count {
        let member = Address::generate(env);
        token_client.mint(&member, &amount);
        members.push(member);
    }
    members
}
