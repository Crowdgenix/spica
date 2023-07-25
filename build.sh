#!/bin/bash

# build all contracts
cargo contract build --manifest-path contracts/ido/Cargo.toml
cargo contract build --manifest-path contracts/ido_factory/Cargo.toml
cargo contract build --manifest-path contracts/token/Cargo.toml
cargo contract build --manifest-path contracts/token_factory/Cargo.toml
cargo contract build --manifest-path contracts/staking/Cargo.toml

# cp artifacts
cp contracts/**/target/ink/*.json artifacts/
cp contracts/**/target/ink/*.contract artifacts/