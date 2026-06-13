<?php

function batchRows(array $rows): Generator
{
    foreach ($rows as $row) {
        yield $row['id'] => $row;
    }
    yield from [];
}
?>
