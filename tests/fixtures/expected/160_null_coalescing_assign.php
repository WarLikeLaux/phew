<?php $options['class'] ??= 'btn btn-default';
$page = $_GET['page'] ?? 1; ?>
<div class="<?= $options['class'] ?>"><?= $page ?></div>
