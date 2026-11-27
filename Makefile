.DEFAULT_GOAL := check
.PHONY: dev lint build check

dev lint build: web/node_modules/.installed .env.example

dev:
	bash scripts/dev.sh

lint:
	cargo fmt --manifest-path server/Cargo.toml --check
	cargo clippy --manifest-path server/Cargo.toml --locked --all-targets -- -D warnings
	npm --prefix web run lint

build:
	npm --prefix web run build
	cargo build --manifest-path server/Cargo.toml --locked --release

check: lint build

web/node_modules/.installed: web/package.json web/package-lock.json
	npm --prefix web ci
	touch $@

.env.example: config/example.env
	cp $< $@
