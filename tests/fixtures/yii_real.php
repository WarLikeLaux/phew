<?php

declare(strict_types=1);

use app\presentation\authors\dto\AuthorEditViewModel;
use yii\helpers\Html;

/**
 * @var AuthorEditViewModel $viewModel
 */

$this->title = Yii::t('app', 'ui.author_update');
$this->params['breadcrumbs'][] = ['label' => Yii::t('app','ui.authors'),'url' => ['index']];
$this->params['breadcrumbs'][] = $this->title;
?>

          <div class="author-update"     >

<h1><?= Html::encode($this->title) ?></h1>

     <?php if($viewModel->hasErrors):?>
                    <div class="alert alert-danger">
  <?= $viewModel->errorSummary ?>
               </div>
<?php endif;?>




               <?php $form = ActiveForm::begin([
        'fieldClass'=>ActiveField::class,
        'options'=>['enctype'=>'multipart/form-data'],
                  'enableAjaxValidation'=>true,
    ]); ?>

    <?= $form->errorSummary($model) ?>

                  <?= $form->field($model,'fio')
        ->textInput(['maxlength'=>true])
        ->withRandomGenerator('fio',['title'=>Yii::t('app','ui.generate_fio')]) ?>

<?= $form->field($model,'year')
                    ->textInput(['type'=>'number'])
        ->withRandomGenerator('year',['title'=>Yii::t('app','ui.generate_year')]) ?>

         <div    class="form-group"   >
                  <span class="form-label"><?= Yii::t('app','ui.current_cover') ?></span><br>
      <?= Html::img($viewModel->coverUrl,['alt'=>$viewModel->title,'style'=>'max-width: 200px;']) ?>
                              </div>

      <?php if(isset($book) && $book->coverUrl):?>
<?= Html::img($book->coverUrl,['alt'=>'cover','class'=>'img-thumbnail']) ?>
            <?php endif;?>

            <table   class="table table-striped"  >
<tr><th><?= Yii::t('app','ui.id') ?></th>
                    <td><?= Html::encode((string)$viewModel->id) ?></td></tr>
    <tr>
                              <th><?= Yii::t('app','ui.fio') ?></th>
<td><?= Html::encode($viewModel->fio) ?></td>
         </tr>
  </table>

                    <div class="form-group">
<?= Html::submitButton(isset($book) ? Yii::t('app','ui.save') : Yii::t('app','ui.create'),['class'=>'btn btn-success']) ?>
    </div>

    <?php ActiveForm::end(); ?>

                              </div>
