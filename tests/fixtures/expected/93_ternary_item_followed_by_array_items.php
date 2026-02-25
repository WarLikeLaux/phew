<?php $menuItems = [
    [
        'items' => [
            YII_ENV_DEV
                ? [
                    'label' => 'Test data',
                    'linkOptions' => ['class' => 'test-data'],
                    'url' => ['/test/index'],
                    'iconClass' => 'bi bi-terminal',
                ]
                : null,
            [
                'label' => 'Journal',
                'linkOptions' => ['class' => 'articles'],
                'url' => ['/internal-article/index'],
                'iconClass' => 'bi bi-newspaper',
            ],
            [
                'label' => 'Instructions',
                'linkOptions' => ['class' => 'instructions'],
                'url' => ['/instruction/index'],
                'iconClass' => 'bi bi-journal-text',
            ],
        ],
        'iconClass' => 'bi bi-list',
    ],
]; ?>
