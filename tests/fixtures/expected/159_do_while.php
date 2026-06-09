<?php $i = 0;
do {
    $i++;
    $total += $items[$i] ?? 0;
} while ($i < $count); ?>
<span><?= $total ?></span>
