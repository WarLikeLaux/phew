<?php if ($model instanceof Product && (Yii::$app->user->can('/*') || Yii::$app->user->can('/product/*') || Yii::$app->user->can('/product/update'))): ?>
    <?= Html::a('Edit', '/product/update/' . $model->id, ['class' => 'btn']); ?>
<?php endif; ?>
<?php if ($currentArt && (Yii::$app->user->can('/*') || Yii::$app->user->can('/arts/*') || Yii::$app->user->can('/arts/update'))): ?>
    <?= Html::a('Edit Art', '/arts/update?id=' . $currentArt->id, ['class' => 'btn']); ?>
<?php endif; ?>
