<div align="center">
<img src="docs/hero.png" alt="phew - fast HTML + PHP formatter for Yii 2 views" width="800">

# PHEW! Your **PH**P vi**EW**s are formatted so quickly!

**⚡ Быстрый форматтер HTML + PHP для view-файлов Yii 2 • Rust 2024 edition**

[![Rust](https://img.shields.io/badge/Rust-2024_Edition-000000?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue?style=flat-square&logo=opensourceinitiative&logoColor=white)](LICENSE)
[![CI](https://img.shields.io/github/actions/workflow/status/WarLikeLaux/phew/ci.yml?style=flat-square&logo=githubactions&logoColor=white&label=CI)](https://github.com/WarLikeLaux/phew/actions/workflows/ci.yml)
[![Clippy](https://img.shields.io/badge/Clippy-0_warnings-brightgreen?style=flat-square&logo=rust&logoColor=white)](https://github.com/WarLikeLaux/phew/actions)
[![Tests](https://img.shields.io/badge/Tests-123_passed-success?style=flat-square&logo=codecov&logoColor=white)](#тестирование)
[![Fixtures](https://img.shields.io/badge/Fixtures-169_pairs-success?style=flat-square&logo=testcafe&logoColor=white)](#тестирование)
[![Version](https://img.shields.io/badge/Version-1.0.0-orange?style=flat-square&logo=semver&logoColor=white)](Cargo.toml)

---

<p align="center">
  <b>🔍 Lexer + AST parser</b> • <b>🎨 HTML + PHP formatting</b> • <b>🔀 Smart line splitting</b><br>
  <b>🏗 Yii 2 widgets</b> • <b>📁 Recursive directory walk</b> • <b>⚙️ Конфиг .phew.toml</b>
</p>

</div>

---

## Зачем

View-файлы Yii 2 - это `.php` с HTML, PHP-вставками, виджетами и альтернативным синтаксисом (`foreach(): ... endforeach;`) вперемешку. Готовые форматтеры с этим не справляются: **Prettier** и **HTMLBeautifier** ломаются на `<?php`, **PHP CS Fixer** не видит HTML и пропускает view-файлы, **Blade Formatter** заточен под Laravel, а **PhpStorm**/**Intelephense** живут только внутри IDE - из консоли, CI или pre-commit хука их не вызвать.

**phew** - один CLI-инструмент, который понимает и HTML, и PHP в контексте друг друга.

## Установка

```bash
cargo install --git https://github.com/WarLikeLaux/phew --force
```

## Использование

```bash
# Вывести отформатированный файл в stdout
phew views/site/index.php

# Перезаписать файл на месте
phew -w views/site/index.php

# Отформатировать всю директорию рекурсивно (.php и .html), обход и форматирование параллельны
# Файлы из .gitignore и .phewignore пропускаются
phew -w views/

# Прочитать из stdin, отформатированный вывести в stdout (для интеграции с редактором)
phew - < views/site/index.php
```

### CI и предпросмотр

```bash
# Режим для CI: ничего не пишет, выходит с кодом ≠0, если файлы не отформатированы
phew --check views/

# Показать, что изменилось бы, без записи (unified diff)
phew --diff views/
```

### Интеграция в проект

Прогнать все вьюхи перед коммитом:

```bash
phew -w views/ widgets/ && git add -u
```

Или как git pre-commit хук (`.git/hooks/pre-commit`):

```sh
#!/bin/sh
phew --check views/ || { echo "Запусти: phew -w views/"; exit 1; }
```

На больших проектах проверяйте только staged-файлы, чтобы не блокировать коммит из-за чужого кода в соседних файлах:

```sh
git diff --cached --name-only -z --diff-filter=d | grep -zE '\.php$' | xargs -0 -r phew --check
```

Отладочные режимы:

```bash
phew --tokens views/site/index.php   # токены лексера
phew --tree views/site/index.php     # AST-дерево
```

Режимы `--write`, `--check`, `--diff`, `--tokens`, `--tree` взаимоисключающие.

## Конфигурация

По умолчанию отступ - 4 пробела, целевая длина строки - 120 символов. Это настраивается через файл `.phew.toml`: phew ищет его от текущего каталога вверх по дереву и останавливается на границе git-репозитория.

```toml
# Стиль отступа: "spaces" или "tabs"
indent_style = "spaces"
# Размер отступа в пробелах (игнорируется при "tabs")
indent_size = 4
# Целевая максимальная длина строки
max_line_length = 120
```

Создать файл с дефолтами:

```bash
phew --init
```

Указать конфиг явно или переопределить отдельные значения флагами:

```bash
phew --config path/to/.phew.toml views/
phew --line-length 100 --indent-style tabs views/
phew --indent-size 2 views/
```

Приоритет: CLI-флаги → `.phew.toml` → значения по умолчанию.

## Пример

**До:** (всё в одну строку, без пробелов, смешанные кавычки)
```php
<div class="catalog">
<?php $form=ActiveForm::begin(['options'=>['data-config'=>'{"ajax":true,"perPage":20}']]);?>
<?=$form->field($model,'category')->dropDownList($categories,['prompt'=>Yii::t('app','Выберите категорию товара')])?>
<?=GridView::widget(['dataProvider'=>$dataProvider,'columns'=>['id',['attribute'=>'type','value'=>fn($m)=>match($m->type){'book'=>Yii::t('app','Книга'),'article'=>Yii::t('app','Статья'),default=>Yii::t('app','Неизвестно')}],['class'=>ActionColumn::class]]])?>
<?php ActiveForm::end();?>
</div>
```

**После:**
```php
<div class="catalog">
    <?php $form = ActiveForm::begin(['options' => ['data-config' => '{"ajax":true,"perPage":20}']]); ?>
        <?= $form->field($model, 'category')
            ->dropDownList($categories, ['prompt' => Yii::t('app', 'Выберите категорию товара')]) ?>
        <?= GridView::widget([
            'dataProvider' => $dataProvider,
            'columns' => [
                'id',
                [
                    'attribute' => 'type',
                    'value' => fn ($m) => match ($m->type) {
                        'book' => Yii::t('app', 'Книга'),
                        'article' => Yii::t('app', 'Статья'),
                        default => Yii::t('app', 'Неизвестно'),
                    },
                ],
                ['class' => ActionColumn::class],
            ],
        ]) ?>
    <?php ActiveForm::end(); ?>
</div>
```

## Что умеет

- **HTML + PHP в едином AST** — отступы вложенных элементов и блоков, alt-синтаксис (`endforeach;`) и brace-стиль
- **Разбивка длинных строк** (≤120) — аргументы, цепочки `->`, массивы, тернарники, `match`
- **Yii 2** — виджеты (`GridView`, `ActiveForm`, `Nav`…), `::begin()`/`::end()`, PHP внутри атрибутов
- **Header-блоки** — PSR-12 порядок `declare → use → docblock`, сортировка/дедуп `use`, `@var`
- **PSR-12 для объявлений** — Allman-брейсы у `class`/`interface`/`trait`/`enum`/`function` (в том числе за атрибутами `#[...]`), развёртка тел методов
- **PHP 8.x** — `match`, `enum`, named args, first-class callable, атрибуты, heredoc/nowdoc
- **Надёжность** — идемпотентность, round-trip, `.gitignore`/`.phewignore`, BOM/CRLF на входе, параллельный обход
- **CI и редакторы** — `--check`, `--diff`, stdin → stdout

Известные ограничения — в [`docs/known-issues.md`](docs/known-issues.md).

## Политика форматирования

| Правило | Значение |
|---------|----------|
| **Целевая длина строки** | ≤120 символов по умолчанию ([настраивается](#конфигурация)) |
| **Исключения** | `<?= ... ?>` echo-блоки, где перенос ухудшает читаемость или ломает выражение |
| **Отступ** | 4 пробела по умолчанию, пробелы или табы ([настраивается](#конфигурация)) |
| **Trailing comma** | Да, в многострочных вызовах |
| **EOF** | Файл заканчивается ровно одним `\n` (POSIX) |

## Тестирование

**123 unit-теста**, **169 fixture-пар** (`tests/fixtures/input/` → `tests/fixtures/expected/`) и **property-тесты** на идемпотентность, round-trip и фаззинг (`tests/properties.rs`). Полная проверка перед коммитом:

```bash
just check          # clippy + test + fixtures
```

## Лицензия

MIT
