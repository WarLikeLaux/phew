<div class="list">
    <?php if ($items): ?>
        <ul>
            <?php foreach ($items as $item): ?>
                <li class="item"><?= $item->name ?></li>
            <?php endforeach; ?>
        </ul>
    <?php else: ?>
    <p class="empty">No items found</p>
<?php endif; ?>
</div>
