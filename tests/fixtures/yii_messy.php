      <div
class="site-index"       >
                     <?php if($model->isActive):?>
  <h1><?= Html::encode($model->title) ?></h1>
            <?php foreach($model->items as $item):?>
                              <div     class="item">
  <?= Html::a($item->name,['item/view','id'=>$item->id],['class'=>'btn btn-primary']) ?>
              <br />
         </div>
<?php endforeach;?>
                  <?php if($model->hasFooter):?>
    <div class="footer"     >
                        <p><?= $model->footer ?></p>
      <a href="/more"    class="btn"   >More</a>
                              </div>
      <?php endif;?>
               <?php endif;?>
                    </div>