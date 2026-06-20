<?php

declare(strict_types=1);

use app\widgets\StatusBadge;
use yii\helpers\Html;

?>
<div class="order-view">
    <h1><?= Html::encode($model->number) ?></h1>
    <?= StatusBadge::widget([
        'status' => $model->status,
        'options' => ['class' => 'order-status', 'data-order-id' => $model->id],
    ]) ?>
</div>
