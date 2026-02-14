<div align="center">
<img src="docs/hero.png" alt="phew - fast HTML + PHP formatter for Yii 2 views" width="800">

# PHEW! Your **PH**P vi**EW**s are formatted so quickly!

**⚡ Быстрый форматтер HTML + PHP для view-файлов Yii 2 • Rust 2024 edition**

[![Rust](https://img.shields.io/badge/Rust-2024_Edition-000000?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue?style=for-the-badge&logo=opensourceinitiative&logoColor=white)](LICENSE)
[![CI](https://img.shields.io/badge/CI-passing-brightgreen?style=for-the-badge&logo=githubactions&logoColor=white)](https://github.com/WarLikeLaux/phew/actions)
[![Clippy](https://img.shields.io/badge/Clippy-0_warnings-brightgreen?style=for-the-badge&logo=rust&logoColor=white)](https://github.com/WarLikeLaux/phew/actions)
[![Tests](https://img.shields.io/badge/Tests-70_passed-success?style=for-the-badge&logo=codecov&logoColor=white)](#тестирование)
[![Fixtures](https://img.shields.io/badge/Fixtures-89_pairs-success?style=for-the-badge&logo=testcafe&logoColor=white)](#тестирование)
[![Version](https://img.shields.io/badge/Version-0.6.0-orange?style=for-the-badge&logo=semver&logoColor=white)](Cargo.toml)

---

<p align="center">
  <b>🔍 Lexer + AST parser</b> • <b>🎨 HTML + PHP formatting</b> • <b>🔀 Smart line splitting</b><br>
  <b>🏗 Yii 2 widgets</b> • <b>📁 Recursive directory walk</b> • <b>⚙️ CLI: fix / check / tokens / tree</b>
</p>

</div>

---

## Зачем

View-файлы в Yii 2 - это `.php`, внутри которых HTML, PHP-вставки, виджеты и альтернативный синтаксис (`foreach(): ... endforeach;`) вперемешку. Ни один из существующих форматтеров не справляется с этим:

- **Prettier** - понимает только HTML. Встретив `<?php`, ломает отступы или выкидывает блок как есть
- **PHP CS Fixer** - работает только с чистым PHP. HTML для него невидим, view-файлы он просто пропускает
- **Blade Formatter** - заточен под Laravel Blade, синтаксис Yii 2 не понимает
- **HTMLBeautifier** - форматирует HTML, но `<?= Html::a(...) ?>` превращает в кашу
- **PhpStorm** - встроенный форматтер лучше всех, но работает только внутри IDE и даже он спотыкается на вложенных виджетах
- **Intelephense** - неплохо справляется с форматированием, но это расширение VS Code. Из консоли, CI или pre-commit хука его не вызовешь

Итого: ты либо форматируешь руками, либо живёшь с кривыми отступами. **phew** закрывает эту дыру - один инструмент, который понимает и HTML, и PHP в контексте друг друга.

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
- ✅ PSR-12 порядок: `declare` → `use` → docblock
- ✅ Алфавитная сортировка `use` statements
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
cargo install --git https://github.com/WarLikeLaux/phew --force
```

## Использование

```bash
# Отформатировать и вывести в stdout
phew views/site/index.php

# Отформатировать и записать в файл
phew -w views/site/index.php

# Отформатировать всю директорию рекурсивно
phew views/

# Записать все изменения в файлы
phew -w views/

# Показать токены (отладка лексера)
phew --tokens views/site/index.php

# Показать AST-дерево (отладка парсера)
phew --tree views/site/index.php

# Вывести версию
phew
```

## Документация

- [Быстрый старт (без глубокого погружения)](docs/quick-start.md)
- [Как работает phew (пайплайн и диаграммы)](docs/how-it-works.md)
- [Техническая архитектура для Rust-разработчиков](docs/rust-developer-guide.md)
- [Гайд для PHP/Yii2-разработчиков](docs/php-developer-guide.md)

## Архитектура

```text
src/
├── main.rs              # CLI (clap): --write, --tokens, --tree
├── lib.rs               # Публичные модули
├── config.rs            # Конфиг (заглушка под .phew.toml)
├── parser/
│   ├── lexer.rs         # Токенизатор HTML + PHP (694 строки)
│   ├── ast.rs           # AST: Element, Text, PhpBlock, PhpEcho (236 строк)
│   └── tree.rs          # Построение дерева (заглушка)
├── formatter/
│   ├── engine.rs        # Оркестрация: emit HTML/PHP, format_nodes (567 строк)
│   ├── indent.rs        # Реиндентация PHP-блоков, нормализация statements (724 строки)
│   ├── split.rs         # Сплиттинг длинных строк, массивы, closure (547 строк)
│   ├── echo.rs          # Форматирование PHP echo: chain, concat, ternary (127 строк)
│   ├── docblock.rs      # Работа с docblock: expand, merge, flush, var normalization (212 строк)
│   ├── php.rs           # PHP: keyword spacing, assignment, fat arrow, splitting (603 строки)
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
| **toml** | Конфиг `.phew.toml` |
| **thiserror** | Типизированные ошибки |
| **anyhow** | Обёртка ошибок в CLI |
| **pretty_assertions** | Читаемые diff-ы в тестах |

## Тестирование

**70 unit-тестов** по всем модулям:

| Модуль | Тестов |
|--------|--------|
| `parser::lexer` | 21 |
| `parser::ast` | 6 |
| `formatter::engine` | 7 |
| `formatter::docblock` | 15 |
| `formatter::php` | 16 |
| stubs (`config`, `parser::tree`, `formatter::html`, `formatter::yii`, `io::walker`, `io::writer`) | 5 |

**89 fixture-пар** (`tests/fixtures/input/` → `tests/fixtures/expected/`):

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
| 52 | `paren_echo` | `<?= ... ?>` внутри скобок и mixed-inline текста |
| 53 | `ternary_echo` | Сложные ternary + длинные Yii-вызовы в echo |
| 54 | `chain_property` | Цепочки + property-access с длинными строковыми литералами |
| 55 | `header_with_if` | Header-блок (`use`, docblock) + `if ... endif` |
| 56 | `register_js_css` | `registerJs/registerCss` с heredoc/многострочными строками |
| 57 | `inline_closure` | Inline-замыкания `function() { ... }` в массивах GridView |
| 58 | `php_comment_close_tag` | `?>` внутри PHP-комментариев |
| 59 | `unclosed_tags` | Незакрытые HTML-теги |
| 60 | `php_in_html_attrs` | PHP внутри HTML-атрибутов (сложный) |
| 61 | `if_else_echo_branches` | if/else ветки с echo |
| 62 | `full_header_block` | Полный header-блок (declare, namespace, use, docblock) |
| 63 | `mixed_echo_block_inline` | Смешанные echo-блоки и inline PHP |
| 64 | `docblock_merge` | Слияние нескольких docblock в один |
| 65 | `use_sorting` | Сортировка use statements по алфавиту |
| 66 | `use_dedup` | Удаление дублей use statements |
| 67 | `use_grouping` | Группировка use по namespace |
| 68 | `header_reorder` | Перестановка declare → use → docblock |
| 69 | `docblock_var_types` | Нормализация @var $name Type → @var Type $name |
| 70 | `trailing_comma_enforce` | Trailing comma в аргументах вызовов |
| 71 | `gridview_action_buttons` | GridView с колонками и кнопками |
| 72 | `form_with_tabs` | Форма с табами |
| 73 | `menu_widget` | Yii2 Menu widget |
| 74 | `dynamic_columns` | Динамическое построение колонок |
| 75 | `widget_options_chain` | Цепочка опций виджета |
| 76 | `php_in_js_data` | PHP-данные внутри JS |
| 77 | `multiline_concat_echo` | Многострочная конкатенация в echo |
| 78 | `nested_ternary` | Вложенные тернарные операторы |
| 79 | `array_of_arrays` | Массив массивов |
| 80 | `mixed_indent_input` | Смешанные отступы на входе |
| 81 | `empty_lines_in_block` | Пустые строки внутри PHP-блока |
| 82 | `php_nowdoc` | PHP Nowdoc синтаксис |
| 83 | `arrow_function_multiline` | Многострочные arrow-функции |
| 84 | `null_coalescing_chain` | Цепочка null coalescing |
| 85 | `match_expression` | PHP match expression |
| 86 | `idempotent_check` | Идемпотентность: повторный прогон |
| 87 | `mixed_echo_styles` | Смешанные стили echo |
| 88 | `consecutive_php_blocks` | Последовательные PHP-блоки |
| 89 | `widget_config_spread` | Spread конфига виджета |

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
| **0.4** | Decompose ≤50 lines, string-aware lexer/engine, uppercase PHP, short tags, textarea RCDATA, echo-in-parens, header+if, registerJs/registerCss, 56 fixtures | ✅ |
| **0.5** | Docblock merge, use sorting, PSR-12 order, decompose engine.rs → 5 modules, 65 fixtures, 66 tests | ✅ |
| **0.6** | Use dedup/grouping, @var normalization, brace/comma breaks, 89 fixtures, 70 tests | ✅ |
| **0.7** | Конфиг `.phew.toml` | 🔜 |
| **1.0** | Стабильный релиз | - |

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
