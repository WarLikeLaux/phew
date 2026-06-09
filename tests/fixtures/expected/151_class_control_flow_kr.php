<?php

class Reducer
{
    public function run(array $items)
    {
        $sum = 0;
        foreach ($items as $item) {
            if ($item > 0) {
                $sum += $item;
            }
        }
        $mapper = function ($x) {
            return $x * 2;
        };
        return $mapper($sum);
    }
}
?>
