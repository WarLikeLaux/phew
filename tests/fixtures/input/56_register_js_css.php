<?php

use yii\web\View;

$this->registerCss("
    .search-results {
        margin-top: 20px;
        padding: 15px;
    }
    .search-results .item {
        border-bottom: 1px solid #eee;
        padding: 10px 0;
    }
");

$this->registerJs(<<<JS
    $(document).on('input', '#search-input', function() {
        $.pjax.reload({
            container: '#search-results',
            url: window.location.href.split('?')[0],
            data: { query: $(this).val() },
            timeout: 10000,
        });
    });
JS, View::POS_READY);

$css = <<<CSS
    .modal-overlay {
        background: rgba(0, 0, 0, 0.5);
        z-index: 1000;
    }
CSS;

$this->registerCss($css);

?>
<div class="content">
    <h1>Test</h1>
</div>
