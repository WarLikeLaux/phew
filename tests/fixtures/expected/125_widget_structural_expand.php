<?= GridView::widget([
    'dataProvider' => $provider,
    'columns' => [
        'id',
        'name',
        'status',
    ],
]) ?>
<?= Breadcrumbs::widget(['links' => $this->params['breadcrumbs']]) ?>
<?= Alert::widget() ?>
<?php $form = ActiveForm::begin(['id' => 'user-form', 'options' => ['class' => 'form-horizontal']]); ?>
<?php ActiveForm::end(); ?>
