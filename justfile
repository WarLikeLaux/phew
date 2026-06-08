default:
    @just --list

# показать список команд
help:
    @just --list

# форматирование + линтер
dev: fmt lint
    @echo "✅ dev ok"

# юнит-тесты (cargo test)
test:
    cargo test

# линтер + тесты + фикстуры
check: lint test fixtures
    @echo "✅ check ok"

# cargo fmt
fmt:
    cargo fmt

# cargo clippy с -D warnings
lint:
    cargo clippy -- -D warnings

# релизная сборка
build:
    cargo build --release

# запуск форматтера
run *args:
    cargo run -- {{args}}

# запуск форматтера с записью (-w)
fix *args:
    cargo run -- -w {{args}}

# фикстурные тесты (input → expected)
fixtures:
    ./bin/check-fixtures

# очистка target/
clean:
    cargo clean

# git diff --staged
dc:
    git diff --staged
