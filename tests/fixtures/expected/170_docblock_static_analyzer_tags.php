<?php

use yii\helpers\Html;

/**
 * @var yii\web\View $this
 * @psalm-var non-empty-list<app\models\User> $users
 * @phpstan-var array<int, app\models\User> $users
 * @template TModel of app\models\User
 * @property-read string $title
 */

?>
<ul>
    <?php foreach ($users as $user): ?>
        <li><?= Html::encode($user->name) ?></li>
    <?php endforeach; ?>
</ul>
