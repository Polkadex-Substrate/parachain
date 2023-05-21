RUSTFLAGS="-D warnings" cargo build --features runtime-benchmarks
./target/debug/parachain-polkadex-node benchmark pallet --pallet "*" --extrinsic "*" --steps 2 --repeat 1
RUSTFLAGS="-D warnings" cargo build
cargo test --workspace
cargo clippy -- -D warnings
cargo fmt --all -- --check