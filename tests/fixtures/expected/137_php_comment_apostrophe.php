<?php // It's a line comment, don't break here
$path = '/admin';
/* We don't close ?> tag inside this comment */
$url = "/x"; ?>
<div>
    <?= $path ?> and <?= $url ?>
</div>
<a data-info="<?= /* it's fine */ $path ?>">link</a>
