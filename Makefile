BIN="main"

.PHONY: run
run:
	cargo run --example $(BIN)

.PHONY: build
build:
	cargo build

