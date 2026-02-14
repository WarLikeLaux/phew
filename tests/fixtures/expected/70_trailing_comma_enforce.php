<?= Html::a(
    Yii::t('app', 'ui.delete'),
    [
        'delete',
        'id' => $model->id,
    ],
    [
        'class' => 'btn btn-danger',
        'data-method' => 'post',
        'data-confirm' => Yii::t('app', 'ui.confirm_delete'),
    ],
) ?>
<?= GridView::widget([
    'dataProvider' => $dataProvider,
    'columns' => [['class' => 'yii\grid\SerialColumn'], 'id', 'name', ['class' => 'yii\grid\ActionColumn']],
]) ?>
