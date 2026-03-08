<?php
$hasContent = !empty($item->number) || !empty($item->article)
    || !empty($item->name) || !empty($item->quantity);

$isValid = $model->isActive()
    && $model->hasPermission()
    && !$model->isDeleted();

$fullName = $user->firstName
    . ' '
    . $user->lastName;
?>
