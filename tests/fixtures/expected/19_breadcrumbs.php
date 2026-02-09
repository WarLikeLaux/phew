<?php

declare(strict_types=1);

use yii\helpers\Html;

$this->title = Yii::t('app', 'ui.books');
$this->params['breadcrumbs'][] = ['label' => Yii::t('app', 'ui.catalog'), 'url' => ['index']];
$this->params['breadcrumbs'][] = ['label' => $model->title, 'url' => ['view', 'id' => $model->id]];
$this->params['breadcrumbs'][] = Yii::t('app', 'ui.update');

?>
<div class="update-page">
    <h1><?= Html::encode($this->title) ?></h1>
    <?= $this->render('_form', ['model' => $model, 'authors' => $authors]) ?>
</div>
