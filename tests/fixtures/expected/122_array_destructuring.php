<?php [$id, $name, $email] = $userTuple;
['lat' => $lat, 'lng' => $lng] = $coordinates; ?>
<ul class="pairs">
    <?php foreach ($pairs as [$key, $value]): ?>
        <li><?= Html::encode($key) ?>: <?= Html::encode($value) ?></li>
    <?php endforeach; ?>
</ul>
