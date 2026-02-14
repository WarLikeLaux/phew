<div class="info">
    <p>
        <?= $model->primaryPhone ?? $model->secondaryPhone ?? $model->contactEmail ?? Yii::t(
            'app',
            'ui.no_contact_info_available_for_this_user',
        ) ?>
    </p>
    <p><?= $model->displayName ?? $model->username ?? $model->email ?? Yii::t('app', 'ui.anonymous') ?></p>
</div>
