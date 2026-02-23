# SoroSave Contracts

Soroban smart contracts for the SoroSave decentralized rotating savings protocol.

## Overview

SoroSave enables trustless group savings (ajo/susu/chit fund) on the Stellar network via Soroban smart contracts.

## Structure

```
contracts/sorosave/src/
├── lib.rs            # Contract entry + constructor
├── types.rs          # Data structures (GroupStatus, SavingsGroup, etc.)
├── errors.rs         # ContractError enum
├── storage.rs        # Storage helpers with TTL management
├── group.rs          # Group lifecycle (create, join, leave, start)
├── contribution.rs   # Contribution logic + token transfers
├── payout.rs         # Payout distribution
├── admin.rs          # Admin controls, disputes, emergency withdraw
└── test.rs           # Unit tests
```

## Build

```bash
cargo build --target wasm32-unknown-unknown --release
```

## Test

```bash
cargo test
```

## License

MIT
