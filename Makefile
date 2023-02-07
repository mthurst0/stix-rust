.PHONY: check \
	build \
	release \
	run \
	test \
	docs

all: check build

check:
	cargo check

build:
	cargo test && cargo build

release:
	cargo build --release

run:
	cargo test && cargo build && cargo run

test:
	cargo test

docs:
	cargo doc --open
