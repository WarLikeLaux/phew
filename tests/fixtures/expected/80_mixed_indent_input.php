<div class="page">
    <h1><?= Html::encode($this->title) ?></h1>
    <p><?= $model->description ?></p>
    <?php if ($model->isActive): ?>
        <div class="alert alert-success"><?= Yii::t('app', 'ui.active') ?></div>
    <?php endif; ?>
</div>
