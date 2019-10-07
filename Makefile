BIN="main"

.PHONY: run
run:
	cargo run --example $(BIN)

.PHONy: test
test:
	cargo test --lib

.PHONY: build
build:
	cargo build

