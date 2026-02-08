<h1><?= $title ?></h1>
<?php $form = ActiveForm::begin(['id' => 'my-form']); ?>
    <div class="form-body">
        <?= $form->field($model, 'name') ?>
        <?= $form->field($model, 'email') ?>
    </div>
<?php ActiveForm::end(); ?>
<footer>
    <?php $total = count($items);
    $label = Yii::t('app', 'items.total'); ?>
    <span><?= "$label: $total" ?></span>
</footer>
