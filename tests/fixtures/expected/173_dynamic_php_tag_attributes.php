<table>
    <thead>
        <tr>
            <th><?= Yii::t('app', 'Label'); ?></th>
            <th <?= $right ? ' style="text-align: right;"' : '' ?>><?= Yii::t('app', 'Value'); ?></th>
            <?php if ($show_details): ?>
                <th><?= Yii::t('app', 'Details'); ?></th>
            <?php endif; ?>
        </tr>
    </thead>
    <tbody>
        <?php foreach ($items as $item): ?>
            <tr>
                <td><?= $item['label'] ?></td>
                <td <?= $right ? ' style="text-align: right;"' : '' ?>><?= $item['value'] ?></td>
                <?php if ($show_details): ?>
                    <td><?= $item['details'] ?></td>
                <?php endif; ?>
            </tr>
        <?php endforeach; ?>
    </tbody>
</table>
