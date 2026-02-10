<?php

use backend\widgets\StatusCircleWidget;
use yii\grid\GridView;
use yii\helpers\Html;

?>

<?= GridView::widget([
    'dataProvider' => $dataProvider,
    'columns' => [
        [
            'label' => '',
            'format' => 'raw',
            'contentOptions' => ['class' => 'w-px-70'],
            'value' => function ($model): string {
                $img = $model->productimgs[0] ?? null;
                if ($img === null) {
                    return '';
                }
                return $this->render(
                    '/common/table/_imageBlock.twig',
                    [
                        'path' => $img->productImage->getAbsolutePath('small'),
                        'previewPath' => $img->productImage->getAbsolutePath('preview'),
                    ],
                );
            },
        ],
        [
            'attribute' => 'name',
            'format' => 'raw',
            'value' => function ($model) {
                if ($model->productType) {
                    return Html::a(
                        $model->productType->name,
                        ['grouped-attrs/update', 'id' => $model->product_type],
                        [
                            'title' => 'Редактировать вид продукции',
                            'target' => '_blank',
                            'data-pjax' => 0,
                        ],
                    );
                }
                return '<span class="text-muted">не задан</span>';
            },
        ],
        [
            'label' => 'Status',
            'value' => fn($model) => StatusCircleWidget::widget(['product' => $model]),
        ],
    ],
]) ?>
