<table class="table">
    <thead>
        <tr>
            <th>ID</th>
            <th>Name</th>
            <th>Actions</th>
        </tr>
    </thead>
    <tbody>
        <?php foreach ($items as $item): ?>
            <tr>
                <td><?= $item->id ?></td>
                <td><?= Html::encode($item->name) ?></td>
                <td>
                    <?= Html::a(
                        '<i class="fa fa-eye"></i>',
                        [
                            'view',
                            'id' => $item->id,
                        ],
                        ['class' => 'btn btn-sm btn-info'],
                    ) ?>
                    <?= Html::a(
                        '<i class="fa fa-edit"></i>',
                        [
                            'update',
                            'id' => $item->id,
                        ],
                        ['class' => 'btn btn-sm btn-primary'],
                    ) ?>
                    <?php if ($item->canDelete()): ?>
                        <?= Html::a(
                            '<i class="fa fa-trash"></i>',
                            [
                                'delete',
                                'id' => $item->id,
                            ],
                            [
                                'class' => 'btn btn-sm btn-danger',
                                'data' => ['method' => 'post', 'confirm' => Yii::t('app', 'Are you sure?')],
                            ],
                        ) ?>
                    <?php endif; ?>
                </td>
            </tr>
        <?php endforeach; ?>
    </tbody>
</table>
