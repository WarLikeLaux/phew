<div class="wizard">
    <?php switch ($step) {
        case 'start': ?>
            <div class="step">
                <h2>Start</h2>
                <p><?= $intro ?></p>
            </div>
            <?php break; ?>
        <?php case 'form': ?>
            <div class="step">
                <h2>Form</h2>
                <?= $this->render('_form', ['model' => $model]) ?>
            </div>
            <?php break; ?>
        <?php default: ?>
            <div class="step">
                <p>Unknown</p>
            </div>
        <?php } ?>
</div>
