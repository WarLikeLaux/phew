<div class="content">
    <?php if ($user->isAdmin): ?>
        <div class="admin-panel">
            <h2><?= Yii::t('app', 'Admin Panel') ?></h2>
            <?php if ($user->isSuperAdmin): ?>
                <span class="badge"><?= $user->role ?></span>
            <?php endif; ?>
        </div>
    <?php elseif ($user->isModerator): ?>
        <div class="mod-panel">
            <h2><?= Yii::t('app', 'Moderator') ?></h2>
            <?= Html::a('Dashboard', ['mod/index'], ['class' => 'btn']) ?>
        </div>
    <?php else: ?>
        <div class="user-panel">
            <?php foreach ($user->notifications as $notification): ?>
                <div class="notification <?= $notification->isRead ? 'read' : 'unread' ?>">
                    <?= Html::encode($notification->message) ?>
                </div>
            <?php endforeach; ?>
        </div>
    <?php endif; ?>
</div>
