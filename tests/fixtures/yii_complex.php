<?php

use backend\widgets\AdminHeaderWidget;
use yii\helpers\Html;
use yii\widgets\Pjax;

/**
 * @var yii\web\View $this
 * @var backend\models\presenters\AdvertisingCampaignPresenter $presenter
 */

$this->title = $presenter->title;
?>
<div class="campaign-index">
    <?= AdminHeaderWidget::widget(['presenter' => $presenter]); ?>
    <div class="title-box">
        <?= Html::a(
            '<i class="bi bi-plus-circle"></i>Добавить кампанию',
            ['create'],
            ['class' => 'btn btn-light-border with-icon']
        ) ?>
        <?= Html::a(
            '<i class="bi bi-collection"></i>Группы рекламных объявлений',
            ['/ad-campaign-group/index'],
            ['class' => 'btn btn-light-border with-icon']
        ) ?>
        <?= Html::a(
            '<i class="bi bi-graph-up"></i>Аналитика',
            ['analytics'],
            ['class' => 'btn btn-light-border with-icon']
        ) ?>
        <?= Html::a(
            '<i class="bi bi-arrow-repeat"></i>Обновить из Яндекс.Директ',
            ['sync'],
            ['class' => 'btn btn-light-border with-icon js-yandex-sync', 'data-url' => \yii\helpers\Url::to(['sync']),]
        ) ?>
    </div>
    <div class="tab-content" id="nav-tabContent">
        <?php foreach ($presenter->tabs as $key => $tab): ?>
            <div class="tab-pane fade show <?= $key === 'all' ? 'active' : '' ?>" id="<?= $tab['id'] ?>" role="tabpanel" aria-labelledby="<?= $tab['id'] ?>-tab">
                <div class="tab-pane">
                    <?php Pjax::begin(['id' => 'campaignPjax-' . $tab['id'], 'options' => ['data-pjax-tab-id' => $tab['id']]]); ?>
                    <?= $this->render(
                        'index/_grid',
                        ['dataProvider' => $tab['dataProvider'], 'searchModel' => $presenter->model, 'emptyText' => $tab['emptyText'],]
                    ); ?>
                    <?php Pjax::end(); ?>
                </div>
            </div>
        <?php endforeach; ?>
    </div>
</div>
