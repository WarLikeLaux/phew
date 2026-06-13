<?= Nav::widget([
    'items' => [
        [
            'label' => Yii::t('app', 'Users'),
            'url' => ['/user/admin/index'],
        ],
        // [
        //     'label' => Yii::t('app', 'Roles'),
        //     'url' => ['/rbac/role/index'],
        // ],
    ],
]) ?>
