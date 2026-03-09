<h1>Файл <?= $type ?> не найден</h1>
<span>Создано <?= Html::encode($model->getCreatedDateText()) ?></span>
<title>Кеш не найден - <?= $feed->name ?></title>
<p>генерации файла <?= $type ?>.</p>
<h2>Взрыв-схема <?= $model->name ? Html::encode($model->name) : "#{$model->id}" ?></h2>
