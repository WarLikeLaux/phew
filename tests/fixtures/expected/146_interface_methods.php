<?php

interface RepositoryInterface
{
    public function find(int $id): ?Model;

    public function save(Model $model): bool;
}
?>
