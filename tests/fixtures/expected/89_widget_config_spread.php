<?= \yii\widgets\DetailView::widget([
    'model' => $model,
    'options' => ['class' => 'table table-striped table-bordered detail-view', 'id' => 'product-detail'],
    'template' => '<tr><th{captionOptions}>{label}</th><td{contentOptions}>{value}</td></tr>',
    'attributes' => [
        'id',
        [
            'attribute' => 'name',
            'label' => Yii::t('app', 'ui.product_name'),
            'format' => 'raw',
            'value' => Html::a(Html::encode($model->name), ['view', 'id' => $model->id]),
        ],
        [
            'attribute' => 'category',
            'label' => Yii::t('app', 'ui.category'),
            'value' => $model->category->name ?? Yii::t('app', 'ui.uncategorized'),
        ],
        [
            'attribute' => 'price',
            'format' => ['currency', 'USD', ['currencyCode' => 'USD']],
            'label' => Yii::t('app', 'ui.price'),
        ],
        [
            'attribute' => 'status',
            'format' => 'raw',
            'value' => Html::tag('span', $model->statusLabel, ['class' => 'badge bg-' . $model->statusColor]),
        ],
        [
            'attribute' => 'tags',
            'format' => 'raw',
            'value' => implode(
                ' ',
                array_map(
                    static fn ($tag) => Html::tag('span', Html::encode($tag), ['class' => 'badge bg-info me-1']),
                    $model->tags,
                ),
            ),
        ],
        [
            'attribute' => 'description',
            'format' => 'ntext',
        ],
        [
            'attribute' => 'created_at',
            'format' => ['date', 'php:d.m.Y H:i'],
        ],
        [
            'attribute' => 'updated_at',
            'format' => ['date', 'php:d.m.Y H:i'],
        ],
        [
            'attribute' => 'author',
            'label' => Yii::t('app', 'ui.author'),
            'value' => $model->author->fullName ?? Yii::t('app', 'ui.unknown'),
        ],
        [
            'attribute' => 'rating',
            'format' => 'raw',
            'value' => Html::tag(
                'div',
                str_repeat('★', $model->rating) . str_repeat('☆', 5 - $model->rating),
                ['class' => 'text-warning'],
            ),
        ],
    ],
]) ?>
