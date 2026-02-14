<div class="<?= $isActive ? 'active' : 'inactive' ?>">
    <a href="<?= Url::to(['site/index', 'id' => $model->id]) ?>" class="<?= $class ?>" data-id="<?= $model->id ?>">
        <?= $model->name ?>
    </a>
</div>
<input type="hidden" name="token" value="<?= Yii::$app->request->csrfToken ?>" />
<div
    id="item-<?= $item->id ?>"
    class="<?= $item->isNew ? 'new' : '' ?> item-card"
    data-url="<?= Url::to(['api/get']) ?>"
>
    <span><?= $item->title ?></span>
</div>
