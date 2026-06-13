<?php echo $form->field($model, 'type')
    ->widget(Select2::class, [
        'theme' => Select2::THEME_DEFAULT,
        'data' => $model->types,
        'options' => ['placeholder' => Yii::t('forms', 'Select a type')],
        'pluginOptions' => [
            'allowClear' => true,
        ],
    ])
; ?>
