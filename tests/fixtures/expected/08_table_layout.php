<table class="table table-striped">
    <tr>
        <th><?= Yii::t('app', 'ui.id') ?></th>
        <td><?= Html::encode($model->id) ?></td>
    </tr>
    <tr>
        <th><?= Yii::t('app', 'ui.name') ?></th>
        <td><?= Html::encode($model->name) ?></td>
    </tr>
    <tr>
        <th><?= Yii::t('app', 'ui.status') ?></th>
        <td>
            <span class="badge badge-<?= $model->statusColor ?>"><?= $model->statusLabel ?></span>
        </td>
    </tr>
</table>
