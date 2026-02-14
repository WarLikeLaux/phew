<?php

use yii\helpers\Html;

/**
 * @var yii\web\View $this
 * @var string $title
 */

$this->title = $title;

$this->params['breadcrumbs'][] = $this->title;

?>
<div class="page">
    <h1><?= Html::encode($this->title) ?></h1>
</div>
