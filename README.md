
## Prerequisities

1. Rust nightly.
2. `cargo contract` with bug fixes for escaping strings in the arguments `cargo install --git https://github.com/paritytech/cargo-contract.git --rev 2b1758756de59bd81e7bed5f8429d364f281cb9a --force`
3. NodeJS >= 14
4. yarn

## Instructions

1. Run local aleph network (easiest is to check out the `aleph-node` repository and run `./scripts/run_nodes.sh` in the root folder).
2. Execute `cargo contract build --manifest-path <path-to-cargo-toml>` script.
3. If successful - contract