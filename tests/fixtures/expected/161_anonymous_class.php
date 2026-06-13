<?php $handler = new class {
    public function run(): int { return 42; }
}; ?>
<div><?= $handler->run() ?></div>
