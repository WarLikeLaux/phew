<div
    class="tab-pane <?= $active ? 'show active' : '' ?>"
    id="<?= $tab->id ?>"
    role="tabpanel"
    aria-labelledby="<?= $tab->id ?>-label"
>
    <div class="<?= $model->hasErrors ? 'alert alert-danger' : 'alert alert-info' ?>"><?= $message ?></div>
    <input type="hidden" name="token" value="<?= $csrf ?>" />
</div>
