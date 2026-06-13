<?php $total = 0;
$add = function ($x) use (&$total) {
    $total += $x;
    return $total;
};
$add(5); ?>
