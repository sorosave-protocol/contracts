# SoroSave Fuzz Targets

This directory configures `cargo-fuzz` targets for the core SoroSave contract flows:

- `create_group`: randomizes boundary values for group creation.
- `contribute`: randomizes member contribution order and repeated contribution attempts.
- `distribute_payout`: randomizes complete and incomplete payout attempts.

Run from `contracts/sorosave`:

```sh
cargo fuzz run create_group
cargo fuzz run contribute
cargo fuzz run distribute_payout
```

The fuzz package is excluded from the root workspace so normal `cargo test` and CI do not pull libFuzzer dependencies.

## Current Findings

No new contract issues are documented yet. These targets establish repeatable fuzz coverage for future edge-case discovery.
