<?php $sorter = function ($a, $b) {
    return $a->order <=> $b->order;
}; ?>
<?= GridView::widget([
    'columns' => [
        [
            'value' => function ($m) {
                return $m->getStatusLabel();
            },
        ],
    ],
]) ?>
