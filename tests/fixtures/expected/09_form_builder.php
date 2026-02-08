<div class="author-form">
    <?php $form = ActiveForm::begin([
        'fieldClass' => ActiveField::class,
        'options' => ['enctype' => 'multipart/form-data'],
        'enableAjaxValidation' => true,
    ]); ?>
    <?= $form->errorSummary($model) ?>
    <?= $form->field($model, 'fio')->textInput(['maxlength' => true]) ?>
    <?= $form->field($model, 'year')->textInput(['type' => 'number']) ?>
    <div class="form-group">
        <?= Html::submitButton(
            isset($book) ? Yii::t('app', 'ui.save') : Yii::t('app', 'ui.create'),
            ['class' => 'btn btn-success']
        ) ?>
    </div>
    <?php ActiveForm::end(); ?>
</div>
