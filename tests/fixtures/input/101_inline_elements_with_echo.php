<div class="feed-info">
    <strong>Фид:</strong> <?= $feed->name ?><br>
    <strong>Тип:</strong> <?= $type ?><br>
    <strong>ID:</strong> <?= $feed->id ?>
</div>
<div class="meta">
    <em>Автор:</em> <?= Html::encode($user->name) ?>,
    <i class="icon-clock"></i> <?= $date ?>
</div>
