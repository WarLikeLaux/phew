default:
    @just --list

help:
    @just --list

alias pf := pack-fixtures

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
    cargo clippy --features web --all-targets -- -D warnings

build:
    cargo build --release

run *args:
    cargo run -- {{args}}

web *args:
    cargo run --features web --bin phew-web -- {{args}}

fix *args:
    cargo run -- -w {{args}}

fixtures:
    ./bin/check-fixtures

pack-fixtures:
    @command -v npx >/dev/null 2>&1 || { echo "npx не установлен"; exit 127; }
    npx -y repomix@1.14.1 --include "tests/fixtures/input/**,tests/fixtures/expected/**" --output packed-fixtures.xml --style xml

clean:
    cargo clean

dc:
    git diff --staged
