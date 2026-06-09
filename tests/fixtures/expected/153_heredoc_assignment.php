<?php $sql = <<<SQL
SELECT id, name
FROM users
WHERE status = :status
SQL;
$rows = Yii::$app->db->createCommand($sql)->queryAll(); ?>
<div><?= count($rows) ?></div>
