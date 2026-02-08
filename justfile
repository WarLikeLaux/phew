default:
    @just --list

dev: fmt lint
    @echo "✅ dev ok"

test:
    cargo test

check: lint test
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

clean:
    cargo clean

d chars="15000":
    ./bin/diff-all {{chars}}

dc:
    git diff --staged
