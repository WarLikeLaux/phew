<?php

use app\models\User;

use yii\helpers\Html;
use yii\widgets\ActiveForm;

?>
<?php $title = $model->isNewRecord
    ? Yii::t('app', 'Create User')
    : Yii::t('app', 'Update User: ') . Html::encode($model->getFullName()); ?>
<?php $this->title = $title;
$this->params['breadcrumbs'][] = ['label' => Yii::t('app', 'Users'), 'url' => ['index']];
$this->params['breadcrumbs'][] = $this->title; ?>
<div class="user-form">
    <h1><?= Html::encode($this->title) ?></h1>
    <?php $form = ActiveForm::begin(['id' => 'user-form', 'options' => ['class' => 'form-horizontal']]); ?>
        <div class="form-body">
            <?= $form->field($model, 'status')
                ->dropDownList(
                    $model->getStatusList(),
                    [
                        'prompt' => $model->isNewRecord
                            ? Yii::t('app', 'Select status for new user account')
                            : Yii::t('app', 'Change current status of existing user account'),
                    ],
                ) ?>
            <?php $buttonLabel = $model->isNewRecord
                ? Yii::t('app', 'Create New User Account Now')
                : Yii::t('app', 'Save Changes To Existing User Account'); ?>
            <div class="form-actions">
                <?= Html::submitButton(
                    $buttonLabel,
                    [
                        'class' => $model->isNewRecord
                            ? 'btn btn-success btn-lg btn-block'
                            : 'btn btn-primary btn-lg btn-block',
                    ],
                ) ?>
            </div>
        </div>
    <?php ActiveForm::end(); ?>
</div>
