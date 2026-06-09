<?= implode(', ', array_map(strtoupper(...), $model->tags)) ?>
<?php $labels = array_map(OrderStatus::from(...), $rawStatuses); ?>
