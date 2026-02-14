<?php

declare(strict_types=1);

use yii\helpers\Html;
use yii\widgets\ActiveForm;
use app\models\User;

/**
 * @var yii\web\View $this
 * @var User $model
 * @var string $title
 */

?>

<?php $this->title = $title; ?>

<div class="user-form">
    <?php $form = ActiveForm::begin(['id' => 'user-form']); ?>
        <?= $form->field($model, 'username') ?>
        <?= $form->field($model, 'email') ?>
        <?= Html::submitButton(Yii::t('app', 'Save'), ['class' => 'btn btn-success']) ?>
    <?php ActiveForm::end(); ?>
</div>
