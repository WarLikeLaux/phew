<div class="page">
    <?php switch ($viewModel->step): case 'intro': ?>
        <div class="step step-intro">
            <h2><?= Yii::t('app', 'ui.step_intro') ?></h2>
            <p><?= Yii::t('app', 'ui.step_intro_text') ?></p>
            <?= Html::a(
                Yii::t('app', 'ui.start'),
                ['wizard/step', 'step' => 'details'],
                ['class' => 'btn btn-primary btn-lg'],
            ) ?>
        </div>
    <?php break; case 'details': ?>
        <div class="step step-details">
            <h2><?= Yii::t('app', 'ui.step_details') ?></h2>
            <?= $this->render('_wizard_form', ['model' => $viewModel->form]) ?>
        </div>
    <?php break; case 'confirm': ?>
        <div class="step step-confirm">
            <h2><?= Yii::t('app', 'ui.step_confirm') ?></h2>
            <?= $this->render('_wizard_summary', ['model' => $viewModel->summary]) ?>
            <?= Html::submitButton(Yii::t('app', 'ui.confirm'), [
                'class' => 'btn btn-success',
                'data-confirm' => Yii::t('app', 'ui.confirm_submit'),
            ]) ?>
        </div>
    <?php break; default: ?>
        <div class="step step-error">
            <p><?= Yii::t('app', 'ui.unknown_step') ?></p>
        </div>
    <?php endswitch; ?>
</div>
