# Дорожная карта

## Сделано

| Версия | Цель |
|--------|------|
| 0.1 | Лексер + базовое форматирование HTML |
| 0.2 | Обработка PHP-блоков, разбивка длинных строк, фикстуры |
| 0.3 | Паттерны Yii 2, нормализация `switch/case`, `::begin()`/`::end()` |
| 0.4 | String-aware лексер/engine, uppercase и short PHP-теги, textarea RCDATA, echo в скобках, header + `if`, `registerJs/registerCss` |
| 0.5 | Слияние docblock, сортировка `use`, PSR-12 порядок, декомпозиция `engine.rs` на модули |
| 0.6.x | Дедуп/сортировка `use`, нормализация `@var`, brace/comma breaks, вложенные массивы, inline run grouping, `visual_len` для кириллицы, spaceship, method chaining |
| 0.7 | Конфиг `.phew.toml` |
| 0.8 | CI-режимы `--check`/`--diff`, stdin → stdout, параллельный обход (rayon), round-trip + property-тесты, `.phewignore`/`.gitignore`, PHP 8.x (`enum`, named args, `f(...)`, `match`, `#[...]`) |
| 0.9 | Рефакторинг под контракт: исчерпывающий `match`, newtypes, `&str`/`Cow`, декомпозиция модулей/функций, гигиена импортов и derive |
| 0.10 | PSR-12 для `class`/`function`: Allman-брейсы, развёртка тел методов с пустой строкой между ними, консистентный `enum` (file-layout PSR-12; `if`/`foreach`/`switch`, замыкания и `match` остаются K&R) |

## План

### 1.0.0 — Стабильный релиз

- Стабилизация формата: вывод не «гуляет» между минорными версиями.

Текущие ограничения форматтера — в [`docs/known-issues.md`](docs/known-issues.md).
