const EXAMPLES = [
    {
        title: "Цикл + if",
        detail: "Карточки заказов, вложенный foreach, условия и echo",
        source: `<section class="orders">
<?php foreach($orders as $order):?>
<article class="order-card" data-id="<?=$order->id?>">
<header>
<h2><?=Html::encode($order->number)?></h2>
<?php if($order->isPaid()):?>
<span class="badge badge-success"><?=Yii::t('app','Оплачен')?></span>
<?php else:?>
<span class="badge badge-warning"><?=Yii::t('app','Ожидает оплаты')?></span>
<?php endif;?>
</header>
<ul class="order-items">
<?php foreach($order->items as $item):?>
<?php if($item->visible):?>
<li data-price="<?=$item->price?>"><?=Html::encode($item->name)?><strong><?=Yii::$app->formatter->asCurrency($item->price)?></strong></li>
<?php endif;?>
<?php endforeach;?>
</ul>
</article>
<?php endforeach;?>
</section>
`,
    },
    {
        title: "GridView",
        detail: "Колонки, ActionColumn, closures и match",
        source: `<?=GridView::widget(['dataProvider'=>$provider,'filterModel'=>$searchModel,'columns'=>['id',['attribute'=>'status','format'=>'raw','value'=>fn($model)=>match($model->status){Order::STATUS_NEW=>Html::tag('span',Yii::t('app','Новый'),['class'=>'badge badge-info']),Order::STATUS_PAID=>Html::tag('span',Yii::t('app','Оплачен'),['class'=>'badge badge-success']),default=>Html::tag('span',Yii::t('app','Архив'),['class'=>'badge badge-secondary'])}],['class'=>ActionColumn::class,'template'=>'{view} {update} {delete}','buttons'=>['update'=>fn($url,$model)=>Html::a(Yii::t('app','Редактировать'),$url,['class'=>'btn btn-sm btn-primary','data-id'=>$model->id])]]]])?>
`,
    },
    {
        title: "ActiveForm",
        detail: "Виджет begin/end, поля, data-атрибуты и подсказки",
        source: `<div class="profile-form">
<?php $form=ActiveForm::begin(['id'=>'profile-form','options'=>['data-controller'=>'profile','data-user-id'=>$model->id]])?>
<?=$form->field($model,'name')->textInput(['maxlength'=>true,'placeholder'=>Yii::t('app','Имя клиента')])?>
<?=$form->field($model,'status')->dropDownList(User::statusLabels(),['prompt'=>Yii::t('app','Выберите статус'),'data-action'=>'change->profile#toggle'])?>
<?php if($model->hasBillingProfile()):?>
<?=$form->field($model,'billing_email')->textInput(['type'=>'email'])?>
<?php endif;?>
<div class="form-actions"><?=Html::submitButton(Yii::t('app','Сохранить'),['class'=>'btn btn-primary'])?></div>
<?php ActiveForm::end()?>
</div>
`,
    },
    {
        title: "DetailView",
        detail: "Массив атрибутов с raw-значениями",
        source: `<?=DetailView::widget(['model'=>$model,'attributes'=>['id','email',['attribute'=>'created_at','format'=>['datetime','php:d.m.Y H:i']],['attribute'=>'manager_id','value'=>fn($model)=>$model->manager?->profile?->name??Yii::t('app','Не назначен')],['attribute'=>'comment','format'=>'raw','value'=>fn($model)=>HtmlPurifier::process($model->comment)]]])?>
`,
    },
    {
        title: "Вложенные виджеты",
        detail: "Pjax, ListView, begin/end и частичные шаблоны",
        source: `<div class="catalog">
<?php Pjax::begin(['id'=>'catalog-pjax','timeout'=>5000])?>
<?=ListView::widget(['dataProvider'=>$dataProvider,'itemView'=>'_card','layout'=>'{summary}<div class="catalog-grid">{items}</div>{pager}','viewParams'=>['currency'=>$currency,'canUpdate'=>Yii::$app->user->can('catalog.update')]])?>
<?php if($dataProvider->count===0):?>
<div class="empty"><?=Yii::t('app','Ничего не найдено')?></div>
<?php endif;?>
<?php Pjax::end()?>
</div>
`,
    },
    {
        title: "Шапка PHP",
        detail: "declare, use и тело view",
        source: `<?php
use yii\\helpers\\Html;
declare(strict_types=1);
use app\\widgets\\StatusBadge;
?>
<div class="order-view">
<h1><?=Html::encode($model->number)?></h1>
<?=StatusBadge::widget(['status'=>$model->status,'options'=>['class'=>'order-status','data-order-id'=>$model->id]])?>
</div>
`,
    },
];

const FORMAT_ENDPOINT = "/api/format";
const AUTO_FORMAT_DELAY_MS = 300;
const IMMEDIATE_FORMAT_DELAY_MS = 0;
const BUSY_TEXT = "Форматирование";
const READY_TEXT = "Готов";
const EMPTY_TEXT = "Пусто";
const COPIED_TEXT = "Скопировано";
const COPY_TEXT = "Копировать";
const COPY_FEEDBACK_MS = 1400;
const ERROR_TEXT = "Ошибка";
const CHANGED_TEXT = "Изменено";
const UNCHANGED_TEXT = "Без изменений";
const PHP_KEYWORDS = new Set([
    "as",
    "class",
    "declare",
    "default",
    "else",
    "elseif",
    "endforeach",
    "endif",
    "false",
    "fn",
    "foreach",
    "function",
    "if",
    "match",
    "new",
    "null",
    "private",
    "protected",
    "public",
    "return",
    "static",
    "strict_types",
    "true",
    "use",
]);
const PHP_TOKEN_PATTERN =
    /('(?:\\.|[^'\\])*'|"(?:\\.|[^"\\])*"|\$[A-Za-z_][A-Za-z0-9_]*|[A-Za-z_][A-Za-z0-9_]*|\b\d+(?:\.\d+)?\b|::|->|\?->|=>|===|!==|==|!=|<=|>=|&&|\|\||[{}\[\]().,;:?=+\-*\/<>])/g;

let activeExampleIndex = 0;

const source = document.querySelector("#source");
const result = document.querySelector("#result");
const sourceStats = document.querySelector("#sourceStats");
const resultStats = document.querySelector("#resultStats");
const status = document.querySelector("#status");
const changed = document.querySelector("#changed");
const copyButton = document.querySelector("#copyButton");
const clearButton = document.querySelector("#clearButton");
const reloadExampleButton = document.querySelector("#reloadExampleButton");
const indentStyle = document.querySelector("#indentStyle");
const indentSize = document.querySelector("#indentSize");
const lineLength = document.querySelector("#lineLength");
const examplesList = document.querySelector("#examplesList");
const exampleCount = document.querySelector("#exampleCount");

let formatTimer = null;
let activeController = null;
let requestSerial = 0;
let formattedResult = "";
let copyTimer = null;

function countLines(value) {
    if (value.length === 0) {
        return 0;
    }
    return value.split(/\r\n|\r|\n/).length;
}

function stats(value) {
    return `${countLines(value)} строк · ${value.length} симв.`;
}

function updateStats() {
    sourceStats.textContent = stats(source.value);
    resultStats.textContent = stats(formattedResult);
}

function setStatus(text, tone) {
    status.textContent = text;
    if (tone) {
        status.dataset.tone = tone;
        return;
    }
    delete status.dataset.tone;
}

function setChanged(isChanged) {
    changed.textContent = isChanged ? CHANGED_TEXT : UNCHANGED_TEXT;
    if (isChanged) {
        changed.dataset.tone = "ok";
        return;
    }
    delete changed.dataset.tone;
}

function setBusy(isBusy) {
    copyButton.disabled = isBusy || formattedResult.length === 0;
    clearButton.disabled = isBusy;
    reloadExampleButton.disabled = isBusy;
    for (const button of examplesList.querySelectorAll("button")) {
        button.disabled = isBusy;
    }
}

function resetCopyButton() {
    if (copyTimer !== null) {
        window.clearTimeout(copyTimer);
        copyTimer = null;
    }
    copyButton.textContent = COPY_TEXT;
    copyButton.classList.remove("copied");
}

function showCopied() {
    if (copyTimer !== null) {
        window.clearTimeout(copyTimer);
    }
    copyButton.textContent = COPIED_TEXT;
    copyButton.classList.add("copied");
    copyTimer = window.setTimeout(resetCopyButton, COPY_FEEDBACK_MS);
}

function span(className, value) {
    return `<span class="${className}">${escapeHtml(value)}</span>`;
}

function escapeHtml(value) {
    return value
        .replaceAll("&", "&amp;")
        .replaceAll("<", "&lt;")
        .replaceAll(">", "&gt;")
        .replaceAll('"', "&quot;")
        .replaceAll("'", "&#39;");
}

function highlightHtmlTag(token) {
    const doctype = token.match(/^<!doctype([\s\S]*)>$/i);
    if (doctype) {
        return `${span("tok-punctuation", "<!")}${span("tok-keyword", "doctype")}${span("tok-text", doctype[1])}${span("tok-punctuation", ">")}`;
    }

    const tag = token.match(/^<(\/?)([A-Za-z][A-Za-z0-9:-]*)([\s\S]*?)(\/?)>$/);
    if (!tag) {
        return span("tok-text", token);
    }

    const [, close, name, attributes, selfClose] = tag;
    if (close) {
        return `${span("tok-punctuation", "</")}${span("tok-tag", name)}${span("tok-punctuation", ">")}`;
    }

    return `${span("tok-punctuation", "<")}${span("tok-tag", name)}${highlightHtmlAttributes(attributes)}${selfClose ? span("tok-punctuation", "/") : ""}${span("tok-punctuation", ">")}`;
}

function highlightHtmlAttributes(value) {
    let html = "";
    let index = 0;

    while (index < value.length) {
        const whitespace = value.slice(index).match(/^\s+/);
        if (whitespace) {
            html += escapeHtml(whitespace[0]);
            index += whitespace[0].length;
            continue;
        }

        const name = value.slice(index).match(/^[^\s=/>]+/);
        if (!name) {
            html += span("tok-punctuation", value[index]);
            index += 1;
            continue;
        }

        html += span("tok-attr", name[0]);
        index += name[0].length;

        if (value[index] !== "=") {
            continue;
        }

        html += span("tok-punctuation", "=");
        index += 1;
        const quote = value[index];
        if (quote === '"' || quote === "'") {
            const end = value.indexOf(quote, index + 1);
            const valueEnd = end === -1 ? value.length - 1 : end;
            html += span("tok-string", value.slice(index, valueEnd + 1));
            index = valueEnd + 1;
            continue;
        }

        const raw = value.slice(index).match(/^[^\s>]+/);
        if (raw) {
            html += span("tok-string", raw[0]);
            index += raw[0].length;
        }
    }

    return html;
}

function findHtmlTagEnd(value, start) {
    let quote = "";
    for (let index = start + 1; index < value.length; index += 1) {
        const char = value[index];
        if (quote) {
            if (char === quote) {
                quote = "";
            }
            continue;
        }
        if (char === '"' || char === "'") {
            quote = char;
            continue;
        }
        if (char === ">") {
            return index;
        }
    }
    return -1;
}

function highlightHtml(value) {
    let html = "";
    let index = 0;

    while (index < value.length) {
        const next = value.indexOf("<", index);
        if (next === -1) {
            html += span("tok-text", value.slice(index));
            break;
        }
        html += span("tok-text", value.slice(index, next));
        const end = findHtmlTagEnd(value, next);
        if (end === -1) {
            html += span("tok-text", value.slice(next));
            break;
        }
        html += highlightHtmlTag(value.slice(next, end + 1));
        index = end + 1;
    }

    return html;
}

function highlightPhpToken(token) {
    if (token.startsWith("'") || token.startsWith('"')) {
        return span("tok-string", token);
    }
    if (token.startsWith("$")) {
        return span("tok-variable", token);
    }
    if (PHP_KEYWORDS.has(token)) {
        return span("tok-keyword", token);
    }
    if (/^\d/.test(token)) {
        return span("tok-number", token);
    }
    if (/^[A-ZА-Я_]/.test(token)) {
        return span("tok-class", token);
    }
    if (/^[A-Za-z_][A-Za-z0-9_]*$/.test(token)) {
        return span("tok-identifier", token);
    }
    return span("tok-operator", token);
}

function highlightPhpInner(value) {
    let html = "";
    let index = 0;

    for (const match of value.matchAll(PHP_TOKEN_PATTERN)) {
        html += span("tok-text", value.slice(index, match.index));
        html += highlightPhpToken(match[0]);
        index = match.index + match[0].length;
    }

    html += span("tok-text", value.slice(index));
    return html;
}

function highlightPhp(value) {
    const open = value.startsWith("<?=") ? "<?=" : value.startsWith("<?php") ? "<?php" : "<?";
    const close = value.endsWith("?>") ? "?>" : "";
    const innerStart = open.length;
    const innerEnd = close ? value.length - close.length : value.length;
    return `${span("tok-php-open", open)}${highlightPhpInner(value.slice(innerStart, innerEnd))}${close ? span("tok-php-close", close) : ""}`;
}

function highlightCode(value) {
    let html = "";
    let index = 0;
    const phpPattern = /<\?(?:php|=)?[\s\S]*?\?>/gi;

    for (const match of value.matchAll(phpPattern)) {
        html += highlightHtml(value.slice(index, match.index));
        html += highlightPhp(match[0]);
        index = match.index + match[0].length;
    }

    html += highlightHtml(value.slice(index));
    return html;
}

function setResult(value) {
    formattedResult = value;
    result.innerHTML = value.length === 0 ? "" : highlightCode(value);
    resetCopyButton();
    updateStats();
}

function cancelFormat() {
    if (formatTimer !== null) {
        window.clearTimeout(formatTimer);
        formatTimer = null;
    }
    if (activeController !== null) {
        activeController.abort();
        activeController = null;
    }
}

function scheduleFormat(delay = AUTO_FORMAT_DELAY_MS) {
    if (activeController !== null) {
        activeController.abort();
        activeController = null;
    }
    if (formatTimer !== null) {
        window.clearTimeout(formatTimer);
    }
    formatTimer = window.setTimeout(formatSource, delay);
}

function options() {
    return {
        indent_style: indentStyle.value,
        indent_size: Number.parseInt(indentSize.value, 10),
        max_line_length: Number.parseInt(lineLength.value, 10),
    };
}

async function parseError(response) {
    const body = await response.json().catch(() => null);
    if (body && typeof body.error === "string") {
        return body.error;
    }
    return `${ERROR_TEXT}: ${response.status}`;
}

async function formatSource() {
    formatTimer = null;

    if (source.value.length === 0) {
        setResult("");
        setChanged(false);
        setStatus(EMPTY_TEXT);
        setBusy(false);
        return;
    }

    setBusy(true);
    setStatus(BUSY_TEXT);
    requestSerial += 1;
    const currentRequest = requestSerial;
    const controller = new AbortController();
    activeController = controller;

    try {
        const response = await fetch(FORMAT_ENDPOINT, {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
            },
            signal: controller.signal,
            body: JSON.stringify({
                source: source.value,
                options: options(),
            }),
        });

        if (!response.ok) {
            throw new Error(await parseError(response));
        }

        const body = await response.json();
        if (currentRequest !== requestSerial) {
            return;
        }
        setResult(body.formatted);
        setChanged(body.changed);
        setStatus(`${READY_TEXT} · ${body.duration_ms.toFixed(1)} мс`, "ok");
    } catch (error) {
        if (error.name === "AbortError") {
            return;
        }
        setStatus(error.message || ERROR_TEXT, "danger");
    } finally {
        if (currentRequest === requestSerial) {
            activeController = null;
            setBusy(false);
        }
    }
}

async function copyResult() {
    if (formattedResult.length === 0) {
        setStatus(EMPTY_TEXT);
        return;
    }
    try {
        await writeClipboard(formattedResult);
        showCopied();
        setStatus(COPIED_TEXT, "ok");
    } catch (error) {
        setStatus(error.message || "Не удалось скопировать", "danger");
    }
}

async function writeClipboard(value) {
    if (navigator.clipboard) {
        await navigator.clipboard.writeText(value);
        return;
    }

    const element = document.createElement("textarea");
    element.value = value;
    element.setAttribute("readonly", "");
    element.style.position = "fixed";
    element.style.top = "-1000px";
    document.body.append(element);
    element.select();
    const copied = document.execCommand("copy");
    element.remove();
    if (!copied) {
        throw new Error("Не удалось скопировать");
    }
}

function clearAll() {
    cancelFormat();
    source.value = "";
    setResult("");
    setChanged(false);
    setStatus(READY_TEXT);
    setBusy(false);
    source.focus();
}

function renderExamples() {
    examplesList.textContent = "";
    exampleCount.textContent = String(EXAMPLES.length);

    EXAMPLES.forEach((example, index) => {
        const button = document.createElement("button");
        const title = document.createElement("span");
        const detail = document.createElement("span");

        button.className = "example-item";
        button.type = "button";
        button.setAttribute("aria-current", index === activeExampleIndex ? "true" : "false");
        title.className = "example-title";
        detail.className = "example-detail";
        title.textContent = example.title;
        detail.textContent = example.detail;

        button.append(title, detail);
        button.addEventListener("click", () => loadExample(index));
        examplesList.append(button);
    });
}

function loadExample(index = activeExampleIndex) {
    activeExampleIndex = index;
    source.value = EXAMPLES[index].source;
    setResult("");
    renderExamples();
    updateStats();
    setChanged(false);
    setStatus(READY_TEXT);
    setBusy(false);
    source.focus();
    scheduleFormat(IMMEDIATE_FORMAT_DELAY_MS);
}

source.addEventListener("input", () => {
    updateStats();
    scheduleFormat();
});
indentStyle.addEventListener("change", () => scheduleFormat());
indentSize.addEventListener("input", () => scheduleFormat());
lineLength.addEventListener("input", () => scheduleFormat());
copyButton.addEventListener("click", copyResult);
clearButton.addEventListener("click", clearAll);
reloadExampleButton.addEventListener("click", () => loadExample(activeExampleIndex));

loadExample();
