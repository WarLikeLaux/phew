<?= Html::encode($model->title) ?>
<div><?= Html::tag('span', $model->name, ['class' => 'label']) ?></div>
<?= $this->render('_partial', ['model' => $model]) ?>
