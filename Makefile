.PHONY: build test lint fmt check ci status

build:
	cargo build --manifest-path crates/Cargo.toml --workspace

test:
	cargo test --manifest-path crates/Cargo.toml --workspace --all-features

lint:
	cargo clippy --manifest-path crates/Cargo.toml --workspace --all-targets --all-features -- -D warnings

fmt:
	cargo fmt --manifest-path crates/Cargo.toml --all

check: fmt lint test

ci:
	cargo fmt --manifest-path crates/Cargo.toml --all -- --check
	cargo clippy --manifest-path crates/Cargo.toml --workspace --all-targets --all-features -- -D warnings
	cargo test --manifest-path crates/Cargo.toml --workspace --all-features

status:
	@./scripts/status.sh
