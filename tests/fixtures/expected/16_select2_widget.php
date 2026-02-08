<?= $form->field($model, 'authorIds')
    ->widget(
        Select2::class,
        [
            'initValueText' => $model->getAuthorInitValueText($authors),
            'options' => ['placeholder' => Yii::t('app', 'ui.placeholder_authors'), 'multiple' => true],
            'bsVersion' => '5',
            'theme' => Select2::THEME_KRAJEE_BS3,
            'pluginOptions' => [
                'allowClear' => true,
                'minimumInputLength' => 2,
                'ajax' => [
                    'url' => Url::to(['author/search']),
                    'dataType' => 'json',
                    'delay' => 250,
                    'data' => new JsExpression('function(params) { return {q:params.term, page:params.page}; }'),
                    'cache' => true,
                ],
            ],
        ],
    )
 ?>
