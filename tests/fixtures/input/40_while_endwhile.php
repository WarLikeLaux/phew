<?php while ($model = $dataReader->read()): ?> <div class="item"> <h3><?= $model['title'] ?></h3> <p><?= $model['description'] ?></p> </div> <?php endwhile; ?>
