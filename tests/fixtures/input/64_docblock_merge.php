<?php

/** @var yii\web\View $this */
/** @var User $model */
/** @var string $title */

use yii\helpers\Html;

/**
 * This is a detailed description
 * of the view file purpose.
 */

/** @var int $count */

if ($count > 0):
?>
<div class="content">
    <h1><?= Html::encode($title) ?></h1>
    <?php
    /** @var Item $item */
    /** @var Category $category */
    foreach ($model->items as $item):
    ?>
        <p><?= $item->name ?></p>
    <?php endforeach; ?>
</div>
<?php endif; ?>
