<?php

use yii\helpers\Html;

?>
<?php /** * @var \common\models\Slider $model */ ?>
<?php if (!$model->img_alias): ?>
    <div class="slide">
        <a href="<?= $model->link ?>">
            <?= '<div class="video-container"><div class="video-slide video-file" data-src="'
                . $model->link
                . '" data-iframe="true">'
                . $model->text
                . '</div></div>' ?>
        </a>
    </div>
<?php endif; ?>
