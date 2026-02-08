<?php

use yii\bootstrap5\Modal;
use yii\helpers\Html;

?>
<div class="page">
    <?php Modal::begin([
        'title' => Yii::t('app', 'ui.modal_title'),
        'id' => 'my-modal',
        'size' => Modal::SIZE_LARGE,
        'options' => ['class' => 'fade', 'tabindex' => -1],
    ]);    echo '<div id="modal-content">' . Yii::t('app', 'ui.loading') . '</div>';
    Modal::end(); ?>
</div>
