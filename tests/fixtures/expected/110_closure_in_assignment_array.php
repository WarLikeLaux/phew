<?php $columns = [
    'id',
    [
        'attribute' => 'status',
        'value' => function ($model) { return $model->getStatusLabel(); },
    ],
    [
        'label' => 'Действия',
        'format' => 'raw',
    ],
]; ?>
