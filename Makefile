BIN="find"

.PHONY: run test build

run:
	cargo run --example $(BIN)

test:
	cargo test --lib

build:
	cargo fmt && \
	cargo clippy && \
	cargo build

