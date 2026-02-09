<div class="form-group">
    <?= Html::submitButton(Yii::t('app', 'ui.save'), ['class' => 'btn btn-success', 'id' => 'save-btn']) ?>
    <?= Html::a(Yii::t('app', 'ui.cancel'), ['index'], ['class' => 'btn btn-secondary ms-2']) ?>
    <?= Html::button(Yii::t('app', 'ui.preview'), [
        'class' => 'btn btn-info ms-2',
        'id' => 'preview-btn',
        'data-bs-toggle' => 'modal',
        'data-bs-target' => '#preview-modal',
    ]) ?>
</div>
