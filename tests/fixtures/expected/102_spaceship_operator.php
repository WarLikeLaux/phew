<?php usort($items, function ($a, $b) {
    return $a->sort <=> $b->sort;
});
usort($data, fn ($a, $b) => $a->position <=> $b->position); ?>
<div>
    <?php usort($list, fn ($a, $b) => $a->order <=> $b->order); ?>
</div>
