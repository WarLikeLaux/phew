<?= $grid->render([
    'valueFormatter' => function ($model = "{default}") {
        return formatModelValueForDisplay($model);
    },
]) ?>
