<?= Html::a(text: Yii::t('app', 'ui.delete_record'), url: ['delete', 'id' => $model->id, 'token' => $csrf], options: ['class' => 'btn btn-danger btn-lg', 'data-method' => 'post']) ?>
