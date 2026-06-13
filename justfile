default:
    @just --list

help:
    @just --list

dev: fmt lint
    @echo "✅ dev ok"

test:
    cargo test

check: lint test fixtures
    @echo "✅ check ok"

fmt:
    cargo fmt

lint:
    cargo clippy --all-targets -- -D warnings

build:
    cargo build --release

run *args:
    cargo run -- {{args}}

fix *args:
    cargo run -- -w {{args}}

fixtures:
    ./bin/check-fixtures

clean:
    cargo clean

dc:
    git diff --staged
