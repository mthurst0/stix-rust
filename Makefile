.PHONY: check \
	build \
	release \
	run

all: check build

check:
	cargo check

build:
	cargo build

release:
	cargo build --release

run:
	cargo run

docs:
	cargo doc --open
