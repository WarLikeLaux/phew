<?php

final class Money {
    public function __construct(public readonly int $amount, public readonly string $currency = 'USD') {}
}

?>
