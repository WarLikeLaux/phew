<?php

use yii\helpers\Html;
use yii\widgets\ActiveForm;

/**
 * @var yii\web\View $this
 * @var app\models\Order $model
 */

?>
<div class="order-form">
    <?php $form = ActiveForm::begin(['id' => 'order-form']); ?>
        <ul class="nav nav-tabs" role="tablist">
            <li class="nav-item">
                <a class="nav-link active" data-bs-toggle="tab" href="#tab-general">
                    <?= Yii::t('app', 'ui.general') ?>
                </a>
            </li>
            <li class="nav-item">
                <a class="nav-link" data-bs-toggle="tab" href="#tab-items"><?= Yii::t('app', 'ui.items') ?></a>
            </li>
            <li class="nav-item">
                <a class="nav-link" data-bs-toggle="tab" href="#tab-shipping"><?= Yii::t('app', 'ui.shipping') ?></a>
            </li>
        </ul>
        <div class="tab-content">
            <div class="tab-pane active" id="tab-general">
                <?= $form->field($model, 'customer_name')->textInput(['maxlength' => true]) ?>
                <?= $form->field($model, 'customer_email')->textInput(['maxlength' => true]) ?>
                <?= $form->field($model, 'notes')->textarea(['rows' => 4]) ?>
            </div>
            <div class="tab-pane" id="tab-items">
                <?php foreach ($model->items as $i => $item): ?>
                    <div class="row mb-2">
                        <div class="col-6">
                            <?= Html::activeTextInput($item, "[$i]product_name", ['class' => 'form-control']) ?>
                        </div>
                        <div class="col-3">
                            <?= Html::activeTextInput(
                                $item,
                                "[$i]quantity",
                                [
                                    'class' => 'form-control',
                                    'type' => 'number',
                                ],
                            ) ?>
                        </div>
                        <div class="col-3">
                            <?= Html::activeTextInput($item, "[$i]price", ['class' => 'form-control']) ?>
                        </div>
                    </div>
                <?php endforeach; ?>
            </div>
            <div class="tab-pane" id="tab-shipping">
                <?= $form->field($model, 'address')->textInput(['maxlength' => true]) ?>
                <?= $form->field($model, 'city')->textInput(['maxlength' => true]) ?>
            </div>
        </div>
        <div class="form-group mt-3">
            <?= Html::submitButton(Yii::t('app', 'ui.save'), ['class' => 'btn btn-success']) ?>
        </div>
    <?php ActiveForm::end(); ?>
</div>
