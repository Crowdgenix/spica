
## Prerequisities

1. Rust nightly.
2. `cargo contract` with bug fixes for escaping strings in the arguments `cargo install --git https://github.com/paritytech/cargo-contract.git --rev 2b1758756de59bd81e7bed5f8429d364f281cb9a --force`
3. NodeJS >= 14
4. yarn

## Instructions

1. Run local aleph network (easiest is to check out the `aleph-node` repository and run `./scripts/run_nodes.sh` in the root folder).
2. Execute `cargo contract build --manifest-path <path-to-cargo-toml>` script.
3. If successful - contract

## Signature explanation
- Why we use signature? Answer: I will give an example, if our project limit max amount of staking for each user, like user A - 1000, user B - 2000 => in this case, we can limit it on the SC, but we can also do it on the BE, we use signature for this case. The BE and the SC will negotiate the signer (the SC will take public key, and the BE will take private key). The flow is user A (or B, C, ...) call the BE API to get the signature, then the BE will use private key to sign the message (in this message, we will add more information that we want to confirm on the SC, like max staking amount, staking duration,...), these information will validate again on the SC using cryptography functionality and that ensure user A is approved by signer (from BE) to use the SC function.