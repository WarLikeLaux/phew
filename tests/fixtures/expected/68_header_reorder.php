<?php

use app\models\User;

use yii\helpers\Html;

declare(strict_types=1);

/**
 * @var yii\web\View $this
 * @var User $model
 */

?>
<div class="page">
    <h1><?= Html::encode($this->title) ?></h1>
    <p><?= $model->name ?></p>
</div>
