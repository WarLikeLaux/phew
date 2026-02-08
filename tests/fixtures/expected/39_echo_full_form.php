<?php echo Html::encode($model->title) ?>
<div>
    <?php echo Html::tag('span', $model->name, ['class' => 'label']) ?>
</div>
<?php echo $this->render('_partial', ['model' => $model]) ?>
