<a href="<?= Url::to(['download-file', 'id' => $model->id]) ?>"
    class="btn btn-accent">
    <i class="feather-icon-download"></i>
    Скачать (<?= $fileExt ?>     <?= $formattedSize ?>)
</a>
<div>
    <p>Вес: <?= $weight ?> кг</p>
    <p>Цена: <?= $price ?> руб. (<?= $discount ?>% скидка)</p>
</div>
