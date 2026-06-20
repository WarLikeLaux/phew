<?php
use yii\helpers\Html;
declare(strict_types=1);
use app\widgets\StatusBadge;
?>
<div class="order-view">
<h1><?=Html::encode($model->number)?></h1>
<?=StatusBadge::widget(['status'=>$model->status,'options'=>['class'=>'order-status','data-order-id'=>$model->id]])?>
</div>
