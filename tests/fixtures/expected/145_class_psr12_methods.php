<?php

class Calculator
{
    private int $total = 0;

    public function add(int $n)
    {
        $this->total += $n;
        return $this;
    }

    public function result(): int
    {
        return $this->total;
    }

    public function __construct() {}
}
?>
