<div>
    <?php

    /**
     * @var array $items
     */

    foreach ($items as $item):
        $label = $item['label'];
        if (!$item['visible']):
            continue;
        endif;

    ?>
        <span><?= $label ?></span>
    <?php endforeach; ?>
</div>
