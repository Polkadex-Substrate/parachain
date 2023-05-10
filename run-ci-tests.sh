cargo build
cargo fmt --all -- --check
cargo clippy -- -D warnings
cargo test --workspace --exclude xcm-simulator