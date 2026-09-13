.PHONY: test build npm-test

test:
	cargo test

build:
	cargo build --release

npm-test:
	npm test --prefix npm
