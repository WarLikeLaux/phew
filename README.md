<div align="center">
<img src="docs/hero.png" alt="phew - fast HTML + PHP formatter for Yii 2 views" width="800">

# PHEW! Your **PH**P vi**EW**s are formatted so quickly!

**⚡ Быстрый форматтер HTML + PHP для view-файлов Yii 2 • Rust 2024 edition**

[![Rust](https://img.shields.io/badge/Rust-2024_Edition-000000?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue?style=for-the-badge&logo=opensourceinitiative&logoColor=white)](LICENSE)
[![CI](https://img.shields.io/badge/CI-passing-brightgreen?style=for-the-badge&logo=githubactions&logoColor=white)](https://github.com/WarLikeLaux/phew/actions)
[![Clippy](https://img.shields.io/badge/Clippy-0_warnings-brightgreen?style=for-the-badge&logo=rust&logoColor=white)](https://github.com/WarLikeLaux/phew/actions)
[![Tests](https://img.shields.io/badge/Tests-49_passed-success?style=for-the-badge&logo=codecov&logoColor=white)](#тестирование)
[![Fixtures](https://img.shields.io/badge/Fixtures-103_pairs-success?style=for-the-badge&logo=testcafe&logoColor=white)](#тестирование)
[![Version](https://img.shields.io/badge/Version-0.6.10-orange?style=for-the-badge&logo=semver&logoColor=white)](Cargo.toml)

---

<p align="center">
  <b>🔍 Lexer + AST parser</b> • <b>🎨 HTML + PHP formatting</b> • <b>🔀 Smart line splitting</b><br>
  <b>🏗 Yii 2 widgets</b> • <b>📁 Recursive directory walk</b> • <b>⚙️ CLI: -w / --tokens / --tree</b>
</p>

</div>

---

## Зачем

View-файлы Yii 2 — это `.php` с HTML, PHP-вставками, виджетами и альтернативным синтаксисом (`foreach(): ... endforeach;`) вперемешку. Готовые форматтеры с этим не справляются: **Prettier** и **HTMLBeautifier** ломаются на `<?php`, **PHP CS Fixer** не видит HTML и пропускает view-файлы, **Blade Formatter** заточен под Laravel, а **PhpStorm**/**Intelephense** живут только внутри IDE — из консоли, CI или pre-commit хука их не вызвать.

**phew** — один CLI-инструмент, который понимает и HTML, и PHP в контексте друг друга.

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

# Отформатировать всю директорию рекурсивно (.php и .html)
phew -w views/
```

### Интеграция в проект

Прогнать все вьюхи перед коммитом:

```bash
phew -w views/ widgets/ && git add -u
```

Или как git pre-commit хук (`.git/hooks/pre-commit`):

```sh
#!/bin/sh
phew -w views/ && git add views/
```

Отладочные режимы:

```bash
phew --tokens views/site/index.php   # токены лексера
phew --tree views/site/index.php     # AST-дерево
```

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

## Что умеет

- ✅ HTML + PHP в едином AST: правильные отступы для вложенных элементов и блоков
- ✅ Альтернативный синтаксис (`if/foreach/for/while/switch ... endforeach;`) и brace-стиль
- ✅ Форматирование PHP: пробелы у ключевых слов, `=>`, запятых, trailing comma
- ✅ Разбивка длинных строк (≤120): по аргументам, цепочкам `->`, вложенным массивам, тернарникам
- ✅ Yii 2: `::begin()`/`::end()` (ActiveForm, Modal, Pjax), GridView, DetailView, Nav, Breadcrumbs, виджеты
- ✅ Inline- и void-элементы, группировка текста + `<?=` на одной строке
- ✅ Header-блоки: PSR-12 порядок `declare → use → docblock`, сортировка и дедуп `use`, нормализация `@var`, слияние docblock
- ✅ PHP внутри HTML-атрибутов с вложенными кавычками (`href="<?= "..." ?>"`)
- ✅ Рекурсивный обход директорий (`.php`, `.html`), идемпотентность, POSIX EOF

## Политика форматирования

| Правило | Значение |
|---------|----------|
| **Целевая длина строки** | ≤120 символов |
| **Исключения** | `<?= ... ?>` echo-блоки, где перенос ухудшает читаемость или ломает выражение |
| **Отступ** | 4 пробела |
| **Trailing comma** | Да, в многострочных вызовах |
| **EOF** | Файл заканчивается ровно одним `\n` (POSIX) |

## Тестирование

**49 unit-тестов** и **103 fixture-пары** (`tests/fixtures/input/` → `tests/fixtures/expected/`). Полная проверка перед коммитом:

```bash
just check          # clippy + test + fixtures
```

## Лицензия

MIT
