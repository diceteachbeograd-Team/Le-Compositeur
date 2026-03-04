SHELL := /bin/sh
CARGO ?= cargo

.PHONY: fmt clippy test check build run-doctor run-init run-preview run-once run-presets run-preset-catalog validate schema ui-blueprint migrate

fmt:
	$(CARGO) fmt --all

clippy:
	$(CARGO) clippy --all-targets --all-features -- -D warnings

test:
	$(CARGO) test --all

check: fmt clippy test

build:
	$(CARGO) build

run-doctor:
	$(CARGO) run -p wc-cli -- doctor

run-init:
	$(CARGO) run -p wc-cli -- init

run-preview:
	$(CARGO) run -p wc-cli -- render-preview

run-once:
	$(CARGO) run -p wc-cli -- run --once

run-presets:
	$(CARGO) run -p wc-cli -- presets

run-preset-catalog:
	$(CARGO) run -p wc-cli -- preset-catalog

validate:
	$(CARGO) run -p wc-cli -- validate

schema:
	$(CARGO) run -p wc-cli -- export-schema

ui-blueprint:
	$(CARGO) run -p wc-cli -- ui-blueprint

migrate:
	$(CARGO) run -p wc-cli -- migrate
