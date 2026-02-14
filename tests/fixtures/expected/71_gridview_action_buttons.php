<?php

use yii\grid\GridView;
use yii\helpers\Html;

/**
 * @var yii\web\View $this
 */

?>
<?= GridView::widget([
    'dataProvider' => $dataProvider,
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
            'class' => 'yii\grid\ActionColumn',
            'template' => '{view} {update} {delete}',
            'buttons' => [
                'view' => static fn ($url, $m) => Html::a(
                    '<i class="bi bi-eye"></i>',
                    $url,
                    [
                        'class' => 'btn btn-sm btn-outline-primary',
                        'title' => Yii::t('app', 'ui.view'),
                    ],
                ),
                'update' => static fn ($url, $m) => Html::a(
                    '<i class="bi bi-pencil"></i>',
                    $url,
                    [
                        'class' => 'btn btn-sm btn-outline-secondary',
                        'title' => Yii::t('app', 'ui.edit'),
                    ],
                ),
                'delete' => static fn ($url, $m) => Html::a(
                    '<i class="bi bi-trash"></i>',
                    $url,
                    [
                        'class' => 'btn btn-sm btn-outline-danger',
                        'data' => ['confirm' => Yii::t('app', 'ui.confirm_delete'), 'method' => 'post'],
                        'title' => Yii::t('app', 'ui.delete'),
                    ],
                ),
            ],
        ],
    ],
]) ?>
