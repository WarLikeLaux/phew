<div class="container">
    <?php if ($user->isAdmin): ?>
        <div class="admin-panel">
            <?php foreach ($sections as $section): ?>
                <div class="section" id="<?= $section->slug ?>">
                    <?php if ($section->hasItems): ?>
                        <ul class="items">
                            <?php foreach ($section->items as $item): ?>
                                <li class="<?= $item->isActive ? 'active' : '' ?>"><?= Html::a($item->title, ['view', 'id' => $item->id]) ?></li>
                            <?php endforeach; ?>
                        </ul>
                    <?php endif; ?>
                </div>
            <?php endforeach; ?>
        </div>
    <?php endif; ?>
</div>
