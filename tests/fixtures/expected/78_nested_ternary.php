<div class="status">
    <?php $label = $model->isPremium
        ? Yii::t('app', 'ui.premium')
        : ($model->isActive ? Yii::t('app', 'ui.active') : Yii::t('app', 'ui.inactive')); ?>
    <span class="badge bg-<?= $model->isPremium ? 'warning' : ($model->isActive ? 'success' : 'secondary') ?>">
        <?= $label ?>
    </span>
</div>
