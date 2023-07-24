SURI=bottom drive obey lake curtain smoke basket hold race lonely fit walk
LOCAL_URL=ws://127.0.0.1:9944
build:
	cargo +nightly-2023-01-17-aarch64-apple-darwin contract build

deploy-local:
    cargo +nightly-2023-01-17-aarch64-apple-darwin contract instantiate --url "$LOCAL_URL" --suri "$SURI" --constructor new --args 1000000 10 -x --skip-dry-run