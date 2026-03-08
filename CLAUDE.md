# CLAUDE.md

## Проект

phew — CLI-форматтер на Rust для PHP/HTML файлов (вьюхи Yii-фреймворка).

## Инструменты

- **just** — task runner, `just help` для списка команд
- **cargo** — сборка и тесты Rust

## Тестирование

- Полная проверка перед коммитом: `just check`
- Фикстурные тесты: пары input/expected в `tests/fixtures/`

## Структура

- `src/` — parser, formatter, io, config
- `tests/fixtures/` — пары input/expected
- `bin/` — вспомогательные скрипты
