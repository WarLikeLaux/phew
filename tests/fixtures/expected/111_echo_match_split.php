<?= match ($model->status) {
    'active' => Yii::t('app', 'ui.status.active'),
    'inactive' => Yii::t('app', 'ui.status.inactive'),
    'pending' => Yii::t('app', 'ui.status.pending'),
    default => Yii::t('app', 'ui.status.unknown')
} ?>
