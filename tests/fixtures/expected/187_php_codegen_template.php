<?php

use yii\helpers\Inflector;

echo "<?php\n";
?>

/**
 * @var yii\web\View $this
 * @var <?php echo ltrim($generator->modelClass, '\\') ?> $model
 */

$this->title = <?php echo $generator->generateString(
    'Create {modelClass}',
    ['modelClass' => Inflector::camel2words($generator->modelClass)],
) ?>;
?>
<div class="<?php echo Inflector::camel2id($generator->modelClass) ?>-create">
    <?php echo "<?php echo " ?>$this->render('_form', [
        'model' => $model,
    ]) ?>
</div>
