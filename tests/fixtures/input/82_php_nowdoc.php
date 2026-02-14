<?php $html = <<<'EOT'
<div class="special">
    <p>This content should NOT be formatted</p>
    <span>  Preserve   whitespace  </span>
</div>
EOT;
?><div class="page"><?= $html ?><p>After nowdoc</p></div>
