<?php

declare(strict_types=1);
use yii\helpers\Url;

?>
<div class="search">
    <div class="row mb-4">
        <div class="col-md-8 offset-md-2">
            <?= $form->field($model, 'query')
                ->textInput([
                    'placeholder' => Yii::t('app', 'ui.search'),
                    'id' => 'search-input',
                    'hx-get' => Url::to(['site/search']),
                    'hx-target' => '#results',
                    'hx-trigger' => 'input changed delay:300ms',
                    'hx-push-url' => 'true',
                    'hx-include' => '#search-form',
                ])

                ->label(false) ?>
        </div>
    </div>
    <div id="results"><?= $this->render('_results', ['dataProvider' => $dataProvider]) ?></div>
</div>
