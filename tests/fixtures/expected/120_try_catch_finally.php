<?php try {
    echo GridView::widget(['dataProvider' => $dataProvider, 'columns' => $columns]);
} catch (\Throwable $e) {
    Yii::error($e->getMessage(), __METHOD__);
    echo Html::tag('div', Yii::t('app', 'ui.render_failed'), ['class' => 'alert alert-danger']);
} finally {
    Yii::endProfile('grid-render');
} ?>
