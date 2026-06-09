<?= GridView::widget([
    'dataProvider' => $provider,
    'columns' => ['identifier', 'name', 'title', 'status', 'createdAt'],
]) ?>
<?= $x ?>Lorem ipsum dolor sit amet
