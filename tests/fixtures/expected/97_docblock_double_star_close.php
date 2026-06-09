<?php

/**
 * @var yii\web\View $this
 * @var common\models\Item $model
 * @var array $links
 */

ItemAsset::register($this);

$this->title = $model->name ? $model->name : "#{$model->id}";
$this->params['breadcrumbs'] = $links;

?>

<div class="container">
    <h1><?= $this->title ?></h1>
</div>
