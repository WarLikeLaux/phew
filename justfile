default:
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
    cargo clippy -- -D warnings

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

d chars="15000":
    ./bin/diff-all {{chars}}

dc:
    git diff --staged

review-fetch *args:
    @node scripts/fetch-pr-comments.mjs {{args}}

review-resolve:
    @node scripts/resolve-pr-threads.mjs
