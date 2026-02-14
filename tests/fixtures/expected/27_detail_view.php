<?= \yii\widgets\DetailView::widget([
    'model' => $model,
    'attributes' => [
        'id',
        [
            'attribute' => 'title',
            'label' => Yii::t('app', 'ui.title'),
        ],
        [
            'attribute' => 'status',
            'format' => 'raw',
            'value' => Html::tag('span', $model->statusLabel, ['class' => 'badge bg-' . $model->statusColor]),
        ],
        [
            'attribute' => 'authorNames',
            'label' => Yii::t('app', 'ui.authors'),
            'value' => implode(', ', $model->authorNames),
        ],
        [
            'attribute' => 'created_at',
            'format' => ['date', 'php:d.m.Y H:i'],
        ],
        [
            'attribute' => 'cover',
            'format' => 'raw',
            'value' => $model->coverUrl
                ? Html::img(
                    $model->coverUrl,
                    [
                        'alt' => $model->title,
                        'style' => 'max-width:200px;',
                        'loading' => 'lazy',
                    ],
                )
                : Yii::t('app', 'ui.no_cover'),
        ],
    ],
]) ?>
