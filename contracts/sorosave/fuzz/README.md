# SoroSave Fuzz Targets

This directory contains `cargo-fuzz` targets for the highest-risk contract flows:

- `create_group`: randomizes contribution amounts, cycle lengths, and member caps, including boundary-style invalid values.
- `contribute`: creates small groups, mints test tokens, starts rounds, and fuzzes member, duplicate, and non-member contribution paths.
- `distribute_payout`: fuzzes complete and incomplete payout rounds and verifies the contract never keeps funds after a successful payout.

Run from `contracts/sorosave`:

```powershell
cargo install cargo-fuzz
cargo fuzz run create_group
cargo fuzz run contribute
cargo fuzz run distribute_payout
```

The seed corpus files under `fuzz/corpus/*` provide minimal and boundary inputs so runs begin with deterministic coverage before libFuzzer mutates them.
