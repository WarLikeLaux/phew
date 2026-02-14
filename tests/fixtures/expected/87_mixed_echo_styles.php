<?= Html::encode($model->title) ?>
<?= $this->render('_partial', ['model' => $model]) ?>
<?= Html::tag('span', $model->name, ['class' => 'label']) ?>
<?= Html::a(Yii::t('app', 'ui.edit'), ['update', 'id' => $model->id], ['class' => 'btn btn-primary']) ?>
