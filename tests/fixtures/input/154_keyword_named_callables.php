<?php $fn = static fn (User $u): string => $u->getFullName(); ?>
<p><?= $fn($user) ?></p>
<p><?= $match($type) ?></p>
<p><?= $formatter->fn($value) ?></p>
