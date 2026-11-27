.DEFAULT_GOAL := check
.PHONY: dev lint build test check
.PHONY: bench-up bench-down bench-seed test-integration
.PHONY: recette-lecture

PYTHON_RECETTE ?= $(shell command -p -v python3 || command -v python3)

dev lint build: web/node_modules/.installed .env.example

dev:
	bash scripts/dev.sh

lint:
	cargo fmt --manifest-path server/Cargo.toml --all --check
	cargo clippy --manifest-path server/Cargo.toml --locked --workspace --all-targets -- -D warnings
	npm --prefix web run lint

build:
	npm --prefix web run build
	cargo build --manifest-path server/Cargo.toml --locked --release

test: web/node_modules/.installed
	cargo test --manifest-path server/Cargo.toml --locked --workspace
	npm --prefix web test

check: lint build test

bench-up:
	docker compose -f bench/docker-compose.yml up --wait --wait-timeout 90

bench-down:
	docker compose -f bench/docker-compose.yml down

bench-seed: export DATASET ?= base
bench-seed: export SEED ?= 42
bench-seed:
	python3 bench/seed.py "$$DATASET" --seed "$$SEED"

test-integration:
	bash bench/test-integration.sh

recette-lecture:
	@cargo build --manifest-path server/Cargo.toml --locked --release --bin webmail-server --target-dir "$${CARGO_TARGET_DIR:-server/target}"
	@"$(PYTHON_RECETTE)" -B bench/recette_lecture.py "$${CARGO_TARGET_DIR:-server/target}/release/webmail-server"

web/node_modules/.installed: web/package.json web/package-lock.json
	npm --prefix web ci
	touch $@

.env.example: config/example.env
	cp $< $@
