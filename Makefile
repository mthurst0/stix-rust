.PHONY: check \
	build \
	release \
	run

all: check build

check:
	cargo check

build:
	cargo test && cargo build

release:
	cargo build --release

run:
	cargo test && cargo build && cargo run

docs:
	cargo doc --open
