<div align="center">
<img src="docs/hero.png" alt="phrust — fast HTML + PHP formatter for Yii 2 views" width="800">

# phrust

**⚡ Быстрый форматтер HTML + PHP для view-файлов Yii 2 • Rust 2024 Edition**

[![Rust](https://img.shields.io/badge/Rust-2024_Edition-000000?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue?style=for-the-badge&logo=opensourceinitiative&logoColor=white)](LICENSE)
[![CI](https://img.shields.io/badge/CI-passing-brightgreen?style=for-the-badge&logo=githubactions&logoColor=white)](https://github.com/WarLikeLaux/phrust/actions)
[![Clippy](https://img.shields.io/badge/Clippy-0_warnings-brightgreen?style=for-the-badge&logo=rust&logoColor=white)](https://github.com/WarLikeLaux/phrust/actions)
[![Tests](https://img.shields.io/badge/Tests-52_passed-success?style=for-the-badge&logo=codecov&logoColor=white)](#тестирование)
[![Fixtures](https://img.shields.io/badge/Fixtures-52_pairs-success?style=for-the-badge&logo=testcafe&logoColor=white)](#тестирование)
[![Version](https://img.shields.io/badge/Version-0.4.2-orange?style=for-the-badge&logo=semver&logoColor=white)](Cargo.toml)

---

<p align="center">
  <b>🔍 Lexer + AST Parser</b> • <b>🎨 HTML + PHP Formatting</b> • <b>🔀 Smart Line Splitting</b><br>
  <b>🏗 Yii 2 Widgets</b> • <b>📁 Recursive Directory Walk</b> • <b>⚙️ CLI: fix / check / tokens / tree</b>
</p>

</div>

---

## Зачем

View-файлы в Yii 2 — это `.php`, внутри которых HTML, PHP-вставки, виджеты и альтернативный синтаксис (`foreach(): ... endforeach;`) вперемешку. Ни один из существующих форматтеров не справляется с этим:

- **Prettier** — понимает только HTML. Встретив `<?php`, ломает отступы или выкидывает блок как есть
- **PHP CS Fixer** — работает только с чистым PHP. HTML для него невидим, view-файлы он просто пропускает
- **Blade Formatter** — заточен под Laravel Blade, синтаксис Yii 2 не понимает
- **HTMLBeautifier** — форматирует HTML, но `<?= Html::a(...) ?>` превращает в кашу
- **PhpStorm** — встроенный форматтер лучше всех, но работает только внутри IDE и даже он спотыкается на вложенных виджетах
- **Intelephense** — неплохо справляется с форматированием, но это расширение VS Code. Из консоли, CI или pre-commit хука его не вызовешь

Итого: ты либо форматируешь руками, либо живёшь с кривыми отступами. **phrust** закрывает эту дыру — один инструмент, который понимает и HTML, и PHP в контексте друг друга.

## Что умеет

- ✅ Парсинг смешанного HTML + PHP в единое AST-дерево
- ✅ Правильные отступы для вложенных HTML-элементов и PHP-блоков
- ✅ Альтернативный синтаксис PHP: `if/elseif/else`, `foreach`, `for`, `while`, `switch/case`
- ✅ Нормализация `switch/case`: split `switch:` / `case` / `break;` / `default:` на отдельные строки
- ✅ Форматирование PHP-кода: пробелы у ключевых слов, `=>`, запятых
- ✅ Разбивка длинных строк (целевой лимит ≤120 символов): по аргументам, цепочкам, вложенным массивам
- ✅ Поддержка Yii 2: `::begin()`/`::end()` пары (ActiveForm, Modal, Pjax и др.), виджеты, `GridView`, `DetailView`, `Nav`, `Breadcrumbs`
- ✅ Inline-элементы (`<span>`, `<a>`, `<strong>` и др.) без переноса на новую строку
- ✅ Void-элементы (`<br>`, `<img>`, `<input>`, `<hr>` и др.)
- ✅ Рекурсивный обход директорий (`.php` и `.html`)
- ✅ Trailing comma в многострочных вызовах
- ✅ Пустая строка после `use`-блока и перед закрывающим `?>`
- ✅ POSIX EOF: файл заканчивается ровно одним `\n`, без лишней пустой строки
- ✅ Header-блоки PHP (declare, namespace, use) с правильным форматированием
- ✅ CLI: `--write`, `--tokens`, `--tree`, поддержка файлов и директорий

## Пример

**До:**
```php
<div class="site-index">
<?php if($model->isActive):?>
<h1><?= Html::encode( $model->title ) ?></h1>
    <?php foreach($model->items as $item):?>
  <div class="item">
        <?= Html::a($item->name,['item/view','id'=>$item->id],['class'=>'btn btn-primary']) ?>
      </div>
<?php endforeach;?>
    <?php endif;?>
</div>
```

**После:**
```php
<div class="site-index">
    <?php if ($model->isActive): ?>
        <h1><?= Html::encode($model->title) ?></h1>
        <?php foreach ($model->items as $item): ?>
            <div class="item">
                <?= Html::a($item->name, ['item/view', 'id' => $item->id], ['class' => 'btn btn-primary']) ?>
            </div>
        <?php endforeach; ?>
    <?php endif; ?>
</div>
```

## Установка

```bash
cargo install --git https://github.com/WarLikeLaux/phrust --force
```

## Использование

```bash
# Отформатировать и вывести в stdout
phrust views/site/index.php

# Отформатировать и записать в файл
phrust -w views/site/index.php

# Отформатировать всю директорию рекурсивно
phrust views/

# Записать все изменения в файлы
phrust -w views/

# Показать токены (отладка лексера)
phrust --tokens views/site/index.php

# Показать AST-дерево (отладка парсера)
phrust --tree views/site/index.php

# Вывести версию
phrust
```

## Архитектура

```text
src/
├── main.rs              # CLI (clap): --write, --tokens, --tree
├── lib.rs               # Публичные модули
├── config.rs            # Конфиг (заглушка под .phrust.toml)
├── parser/
│   ├── lexer.rs         # Токенизатор HTML + PHP (629 строк)
│   ├── ast.rs           # AST: Element, Text, PhpBlock, PhpEcho (234 строк)
│   └── tree.rs          # Построение дерева (заглушка)
├── formatter/
│   ├── engine.rs        # Движок форматирования (829 строк)
│   ├── php.rs           # PHP: keyword spacing, fat arrow, commas, splitting (425 строк)
│   ├── html.rs          # HTML-правила (заглушка)
│   └── yii.rs           # Yii 2 паттерны (заглушка)
└── io/
    ├── walker.rs        # Обход файлов (заглушка)
    └── writer.rs        # Запись файлов (заглушка)
```

**Пайплайн:**  `Input → Lexer (tokens) → AST Parser (tree) → Formatter Engine → Output`

## Технологии

| Технология | Зачем |
|------------|-------|
| **Rust** | Скорость, безопасная работа с памятью, один бинарник без зависимостей |
| **clap** | Парсинг CLI-аргументов |
| **toml** | Конфиг `.phrust.toml` |
| **thiserror** | Типизированные ошибки |
| **anyhow** | Обёртка ошибок в CLI |
| **pretty_assertions** | Читаемые diff-ы в тестах |

## Тестирование

**52 unit-теста** по всем модулям:

| Модуль | Тестов |
|--------|--------|
| `parser::lexer` | 21 |
| `parser::ast` | 6 |
| `formatter::engine` | 7 |
| `formatter::php` | 11 |
| stubs (config, html, yii, io) | 7 |

**51 fixture-пары** (`tests/fixtures/input/` → `tests/fixtures/expected/`):

| # | Фикстура | Что тестирует |
|---|----------|---------------|
| 01 | `html_div` | Чистый HTML (.html) |
| 02 | `html_attrs` | HTML-атрибуты (.html) |
| 03 | `echo` | PHP echo-блоки |
| 04 | `control_flow` | if/elseif/else/endif |
| 05 | `chain` | Цепочки вызовов `->` |
| 06 | `args_split` | Разбивка длинных аргументов |
| 07 | `php_attrs` | PHP внутри HTML-атрибутов |
| 08 | `table` | Таблица с вложенным PHP |
| 09 | `active_form` | ActiveForm::begin/end |
| 10 | `compact` | Компактный PHP-блок |
| 11 | `blank_lines` | Пустые строки |
| 12 | `nesting` | Глубокая вложенность |
| 13 | `header` | Header PHP-блок (declare, use) |
| 14 | `begin_end` | beginTag/endTag |
| 15 | `gridview` | GridView с вложенными массивами |
| 16 | `nested_array` | Select2 с глубокими массивами |
| 17 | `ternary` | Тернарные операторы |
| 18 | `modal` | Modal виджет |
| 19 | `breadcrumbs` | Breadcrumbs |
| 20 | `data_attrs` | data-атрибуты |
| 21 | `field_config` | Конфигурация полей |
| 22 | `htmx` | HTMX-атрибуты |
| 23 | `submit_group` | Группа submit-кнопок |
| 24 | `nested_if` | Вложенные if/elseif |
| 25 | `pjax_list` | Pjax со списками |
| 26 | `foreach_cards` | foreach с карточками |
| 27 | `detail_view` | DetailView виджет |
| 28 | `nav_items` | Nav с подменю |
| 29 | `inline_loop` | Inline PHP в циклах |
| 30 | `switch_case` | switch/case/default |
| 31 | `script_raw_text` | JS в `<script>` (raw-text) |
| 32 | `style_raw_text` | CSS в `<style>` (raw-text) |
| 33 | `doctype` | `<!DOCTYPE>` |
| 34 | `html_comments` | `<!-- -->` комментарии |
| 35 | `brace_if_else` | Brace-style if/else |
| 36 | `brace_foreach` | Brace-style foreach |
| 37 | `for_while_alt` | for/while alt-syntax |
| 38 | `brace_for_while` | Brace-style for/while |
| 39 | `echo_full_form` | `<?php echo ?>` full form |
| 40 | `while_endwhile` | while/endwhile |
| 41 | `mid_html_php` | PHP в середине HTML |
| 42 | `nested_widget` | Вложенные widget begin/end |
| 43 | `empty_file` | Пустой файл |
| 44 | `text_only` | Текст без тегов |
| 45 | `brace_switch` | Brace-style switch/case |
| 46 | `php_close_tag_inside_string` | `?>` внутри PHP-строк |
| 47 | `break_in_string_no_dedent` | `break;` в строковом литерале |
| 48 | `uppercase_php_open_tag` | `<?PHP` uppercase |
| 49 | `short_open_tag` | `<? ... ?>` short tag |
| 50 | `textarea_rcdata` | RCDATA для `<textarea>` (без парсинга HTML внутри) |
| 51 | `inline_mixed_text_inline_tag` | Смешанный текст + inline-теги |

```bash
# Unit-тесты
just test           # или cargo test

# Fixture-тесты
just fixtures       # или ./bin/check-fixtures
```

## Утилиты

| Команда | Описание |
|---------|----------|
| `just dev` | fmt + clippy |
| `just test` | cargo test |
| `just check` | clippy + test + fixtures |
| `just fixtures` | Проверка fixture-пар |
| `just build` | Релизная сборка |
| `just run <args>` | Запуск с аргументами |
| `just fix <args>` | Форматирование с записью |
| `just d [chars]` | Diff всех изменений |
| `just review-fetch` | Получить комментарии из PR |
| `just review-resolve` | Закрыть треды на GitHub |

## Дорожная карта

| Фаза | Цель | Статус |
|------|------|--------|
| **0.1** | Лексер + базовое форматирование HTML | ✅ |
| **0.2** | Обработка PHP-блоков, line splitting, fixtures | ✅ |
| **0.3** | Паттерны Yii 2, switch/case normalization, ::begin/::end, 45 fixtures | ✅ |
| **0.4** | Decompose ≤50 lines, string-aware lexer/engine, uppercase PHP, short tags, textarea RCDATA, 51 fixtures | ✅ |
| **0.5** | Конфиг `.phrust.toml` | 🔜 |
| **1.0** | Стабильный релиз | — |

## Политика форматирования

| Правило | Значение |
|---------|----------|
| **Целевая длина строки** | ≤120 символов |
| **Исключения** | `<?= ... ?>` echo-блоки, где перенос ухудшает читаемость или ломает выражение |
| **EOF** | Файл заканчивается ровно одним `\n` (POSIX). Лишняя пустая строка `\n\n` недопустима |
| **Отступ** | 4 пробела |
| **Trailing comma** | Да, в многострочных вызовах |

## CI

GitHub Actions: `fmt → clippy → test → fixtures → build` на каждый push и PR в `main`.

## Лицензия

MIT
