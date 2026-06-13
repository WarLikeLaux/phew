<div>
    <?= GridView::widget([
        'dataProvider' => $dataProvider,
        'columns' => [
            [
                'attribute' => 'name',
                'content' => function ($model) {
                    /** @var User $model */
                    return $model->name;
                },
            ],
            [
                'class' => ActionColumn::class,
                //'updateOptions' => ['label' => 'Edit'],
                'viewOptions' => ['label' => 'View'],
            ],
        ],
    ]) ?>
</div>
