# Набросок дизайна phrust

> ⚠️ Это черновик. Ничего из описанного ниже пока не реализовано - это предположения о том, как инструмент может работать. Всё будет меняться по ходу разработки.

## Примерная архитектура

Идея в том, чтобы разбить на три слоя: парсинг, форматирование, ввод-вывод.

```
src/
├── main.rs            # Точка входа CLI (clap)
├── config.rs          # Парсинг .phrust.toml
├── parser/
│   ├── mod.rs
│   ├── lexer.rs       # Токенизатор смешанного HTML + PHP
│   ├── ast.rs         # Определения узлов AST
│   └── tree.rs        # Построение дерева
├── formatter/
│   ├── mod.rs
│   ├── engine.rs      # Основной пайплайн форматирования
│   ├── html.rs        # Правила для HTML
│   ├── php.rs         # Правила для PHP-блоков
│   └── yii.rs         # Паттерны Yii 2
└── io/
    ├── mod.rs
    ├── walker.rs      # Обход файлов + .phrustignore
    └── writer.rs      # Запись: in-place / stdout
```

## Предполагаемый конфиг

Пока думаю про TOML. Что-то такое:

```toml
[format]
indent_size = 4
indent_style = "space"       # "space" | "tab"
max_line_length = 120
self_closing_style = "xhtml" # "xhtml" (<br />) | "html5" (<br>)

[format.html]
sort_attributes = true
attribute_style = "double"   # "double" | "single"

[format.php]
short_open_tag = true        # <?= вместо <?php echo
blank_lines_after_open = 0

[yii]
framework_version = "2.0"
```

## Пример форматирования

Как это должно выглядеть в идеале.

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

## CLI-команды (предварительно)

```bash
phrust fix .                       # форматировать всё
phrust fix views/site/index.php    # один файл
phrust check .                     # проверка без изменений (для CI)
phrust diff views/                 # показать что изменится
cat view.php | phrust --stdin      # из stdin
```

## Открытые вопросы

- Писать парсер руками или взять `tree-sitter`?
- Нужен ли LSP или хватит простого CLI?
- PHP-форматирование - делать самим или делегировать в `php-cs-fixer` для чистых блоков?
- Насколько глубоко лезть в Yii-специфику (виджеты, GridView и т.д.)?
