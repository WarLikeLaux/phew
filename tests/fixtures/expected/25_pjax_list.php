<?php

declare(strict_types=1);
use yii\helpers\Html;
use yii\widgets\Pjax;
use yii\grid\GridView;
use yii\bootstrap5\LinkPager;

?>
<div class="list-page">
    <h1><?= Html::encode($this->title) ?></h1>
    <?php Pjax::begin(['id' => 'list-pjax', 'timeout' => 5000]); ?>
    <?= GridView::widget([
        'dataProvider' => $dataProvider,
        'filterModel' => $searchModel,
        'pager' => ['class' => LinkPager::class],
        'columns' => [
            ['class' => 'yii\grid\SerialColumn'],
            [
                'attribute' => 'name',
                'label' => Yii::t('app', 'ui.name'),
                'format' => 'raw',
                'value' => static fn ($m) => Html::a(Html::encode($m->name), ['view', 'id' => $m->id]),
            ],
            [
                'attribute' => 'status',
                'filter' => $statuses,
                'value' => static fn ($m) => $m->statusLabel,
            ],
            [
                'attribute' => 'created_at',
                'format' => 'datetime',
            ],
            [
                'class' => 'yii\grid\ActionColumn',
                'template' => '{view} {update} {delete}',
                'buttons' => ['delete' => static fn ($url, $m) => Html::a('<i class="bi bi-trash"></i>', $url, ['data' => ['confirm' => Yii::t('app', 'ui.confirm_delete'), 'method' => 'post']])],
            ],
        ],
    ])
 ?>
    <?php Pjax::end(); ?>
</div>
