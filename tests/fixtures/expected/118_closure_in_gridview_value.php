<?= GridView::widget([
    'dataProvider' => $dataProvider,
    'columns' => [
        'id',
        [
            'attribute' => 'status',
            'value' => function ($model) {
                return $model->getStatusLabel();
            },
        ],
        ['class' => 'yii\grid\ActionColumn'],
    ],
]) ?>
