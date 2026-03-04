fmt:
    cargo fmt --all

clippy:
    cargo clippy --all-targets --all-features -- -D warnings

test:
    cargo test --all

check: fmt clippy test

build:
    cargo build

run-doctor:
    cargo run -p wc-cli -- doctor

run-init:
    cargo run -p wc-cli -- init

run-preview:
    cargo run -p wc-cli -- render-preview

run-once:
    cargo run -p wc-cli -- run --once

run-presets:
    cargo run -p wc-cli -- presets

run-preset-catalog:
    cargo run -p wc-cli -- preset-catalog

validate:
    cargo run -p wc-cli -- validate

schema:
    cargo run -p wc-cli -- export-schema

ui-blueprint:
    cargo run -p wc-cli -- ui-blueprint

migrate:
    cargo run -p wc-cli -- migrate
