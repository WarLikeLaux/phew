<?= GridView::widget([
    'dataProvider' => $dataProvider,
    'columns' => [['attribute' => 'type', 'value' => static fn ($m) => match ($m->type) { 'book' => Yii::t('app', 'ui.book'), 'article' => Yii::t('app', 'ui.article'), 'review' => Yii::t('app', 'ui.review'), default => Yii::t('app', 'ui.unknown') }]],
]) ?>
