<a
    href='<?= "/admin/instruction/update?id={$model->id}" ?>'
    target="_blank"
    class="tag-list__item m-0 ml-px-12 text-muted"
>
    Изменить
</a>
<a href="<?= Url::to(['view', 'id' => $model->id]) ?>" class="btn">View</a>
<img src="<?= '/images/' . $model->photo ?>" alt="<?= $model->name ?>">
