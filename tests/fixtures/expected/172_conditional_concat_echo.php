<?= 'Status: '
    . ($model->active ? 'active' : 'blocked')
    . ' / '
    . ($model->count > 0 ? Yii::t('app', 'Has items') : Yii::t('app', 'Empty')) ?>
