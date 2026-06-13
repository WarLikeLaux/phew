<?php

function formatPrice(float $value, string $currency = 'USD'): string
{
    return number_format($value, 2) . ' ' . $currency;
}
?>
