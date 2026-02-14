<div class="page">
<?php
// This is a comment
$x = 1;
?>
<p><?= $x ?></p>
<?php
/* Multi-line
   comment here */
$y = $x + 1;
?>
<p><?= $y ?></p>
<?php
$url = '/path?param=value';
$query = "SELECT * FROM t WHERE x > 0";
?>
<span><?= $url ?></span>
</div>
