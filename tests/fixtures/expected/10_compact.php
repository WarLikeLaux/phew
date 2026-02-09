<?php

use app\models\User;
use app\helpers\Html;
use yii\widgets\ActiveForm;

/**
 * @var yii\web\View $this
 * @var User $model
 * @var string $title
 */

$this->title = $title;
$this->params['breadcrumbs'][] = ['label' => 'Users', 'url' => ['index']];
$this->params['breadcrumbs'][] = $this->title;

?>
<div class="user-update">
    <h1><?= Html::encode($this->title) ?></h1>
</div>
