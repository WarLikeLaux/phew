<?php

use yii\helpers\Html;
use yii\widgets\ActiveForm;

$this->title = 'Create';
$this->params['breadcrumbs'][] = $this->title;

?>
<div class="author-form">
    <?= $form->field($model, 'fio') ?>
</div>
