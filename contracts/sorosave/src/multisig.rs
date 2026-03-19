use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Address, Env, Map, Symbol, Vec,
};

#[derive(Clone)]
#[contracttype]
pub struct Proposal {
    pub id: u32,
    pub proposer: Address,
    pub target: Address,
    pub function_name: Symbol,
    pub args: Vec<soroban_sdk::Val>,
    pub approvals: Vec<Address>,
    pub executed: bool,
    pub created_at: u64,
    pub expires_at: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct MultisigConfig {
    pub signers: Vec<Address>,
    pub threshold: u32,
    pub proposal_timeout: u64,
}

const PROPOSALS: Symbol = symbol_short!("PROPOSALS");
const CONFIG: Symbol = symbol_short!("CONFIG");
const PROPOSAL_COUNT: Symbol = symbol_short!("P_COUNT");

#[contract]
pub struct MultisigContract;

#[contractimpl]
impl MultisigContract {
    pub fn initialize(env: Env, signers: Vec<Address>, threshold: u32, proposal_timeout: u64) {
        if threshold == 0 || threshold > signers.len() {
            panic!("Invalid threshold");
        }

        let config = MultisigConfig {
            signers,
            threshold,
            proposal_timeout,
        };

        env.storage().instance().set(&CONFIG, &config);
        env.storage().instance().set(&PROPOSAL_COUNT, &0u32);
    }

    pub fn create_proposal(
        env: Env,
        proposer: Address,
        target: Address,
        function_name: Symbol,
        args: Vec<soroban_sdk::Val>,
    ) -> u32 {
        proposer.require_auth();

        let config: MultisigConfig = env.storage().instance().get(&CONFIG).unwrap();
        
        if !config.signers.contains(&proposer) {
            panic!("Not authorized signer");
        }

        let proposal_count: u32 = env.storage().instance().get(&PROPOSAL_COUNT).unwrap_or(0);
        let proposal_id = proposal_count + 1;

        let current_time = env.ledger().timestamp();
        let expires_at = current_time + config.proposal_timeout;

        let proposal = Proposal {
            id: proposal_id,
            proposer: proposer.clone(),
            target,
            function_name,
            args,
            approvals: Vec::from_array(&env, [proposer]),
            executed: false,
            created_at: current_time,
            expires_at,
        };

        let mut proposals: Map<u32, Proposal> = env
            .storage()
            .persistent()
            .get(&PROPOSALS)
            .unwrap_or(Map::new(&env));

        proposals.set(proposal_id, proposal);
        env.storage().persistent().set(&PROPOSALS, &proposals);
        env.storage().instance().set(&PROPOSAL_COUNT, &proposal_id);

        proposal_id
    }

    pub fn approve_proposal(env: Env, proposal_id: u32, signer: Address) {
        signer.require_auth();

        let config: MultisigConfig = env.storage().instance().get(&CONFIG).unwrap();
        
        if !config.signers.contains(&signer) {
            panic!("Not authorized signer");
        }

        let mut proposals: Map<u32, Proposal> = env
            .storage()
            .persistent()
            .get(&PROPOSALS)
            .unwrap_or(Map::new(&env));

        let mut proposal = proposals.get(proposal_id).unwrap();

        if proposal.executed {
            panic!("Proposal already executed");
        }

        if env.ledger().timestamp() > proposal.expires_at {
            panic!("Proposal expired");
        }

        if proposal.approvals.contains(&signer) {
            panic!("Already approved");
        }

        proposal.approvals.push_back(signer);
        proposals.set(proposal_id, proposal);
        env.storage().persistent().set(&PROPOSALS, &proposals);
    }

    pub fn execute_proposal(env: Env, proposal_id: u32, executor: Address) {
        executor.require_auth();

        let config: MultisigConfig = env.storage().instance().get(&CONFIG).unwrap();
        
        if !config.signers.contains(&executor) {
            panic!("Not authorized signer");
        }

        let mut proposals: Map<u32, Proposal> = env
            .storage()
            .persistent()
            .get(&PROPOSALS)
            .unwrap_or(Map::new(&env));

        let mut proposal = proposals.get(proposal_id).unwrap();

        if proposal.executed {
            panic!("Proposal already executed");
        }

        if env.ledger().timestamp() > proposal.expires_at {
            panic!("Proposal expired");
        }

        if proposal.approvals.len() < config.threshold {
            panic!("Insufficient approvals");
        }

        proposal.executed = true;
        proposals.set(proposal_id, proposal.clone());
        env.storage().persistent().set(&PROPOSALS, &proposals);

        // Execute the proposal
        env.invoke_contract(
            &proposal.target,
            &proposal.function_name,
            proposal.args,
        );
    }

    pub fn get_proposal(env: Env, proposal_id: u32) -> Option<Proposal> {
        let proposals: Map<u32, Proposal> = env
            .storage()
            .persistent()
            .get(&PROPOSALS)
            .unwrap_or(Map::new(&env));

        proposals.get(proposal_id)
    }

    pub fn get_config(env: Env) -> MultisigConfig {
        env.storage().instance().get(&CONFIG).unwrap()
    }

    pub fn get_proposal_count(env: Env) -> u32 {
        env.storage().instance().get(&PROPOSAL_COUNT).unwrap_or(0)
    }

    pub fn is_signer(env: Env, address: Address) -> bool {
        let config: MultisigConfig = env.storage().instance().get(&CONFIG).unwrap();
        config.signers.contains(&address)
    }
}