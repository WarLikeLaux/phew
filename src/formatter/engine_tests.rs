use super::*;
use crate::parser::{ast, lexer};
use pretty_assertions::assert_eq;

fn format_str(input: &str) -> String {
    let tokens = lexer::tokenize(input);
    let nodes = ast::parse(tokens);
    Formatter::default().format(&nodes)
}

#[test]
fn simple_div() {
    assert_eq!(format_str("<div>hello</div>"), "<div>hello</div>\n");
}

#[test]
fn nested_html() {
    let input = "<div><p>text</p></div>";
    let expected = "\
<div>
    <p>text</p>
</div>
";
    assert_eq!(format_str(input), expected);
}

#[test]
fn self_closing_tag() {
    assert_eq!(format_str("<br />"), "<br>\n");
}

#[test]
fn php_echo_inline() {
    let input = "<h1><?= $title ?></h1>";
    assert_eq!(format_str(input), "<h1><?= $title ?></h1>\n");
}

#[test]
fn php_block_indentation() {
    let input = "<div><?php if ($x): ?><p>yes</p><?php endif; ?></div>";
    let expected = "\
<div>
    <?php if ($x): ?>
        <p>yes</p>
    <?php endif; ?>
</div>
";
    assert_eq!(format_str(input), expected);
}

#[test]
fn attributes_preserved() {
    let input = r#"<div class="container" id="main"><p>hi</p></div>"#;
    let expected = "\
<div class=\"container\" id=\"main\">
    <p>hi</p>
</div>
";
    assert_eq!(format_str(input), expected);
}

#[test]
fn nested_php_blocks() {
    let input = "<div><?php if ($a): ?><?php foreach ($items as $i): ?><p><?= $i ?></p><?php endforeach; ?><?php endif; ?></div>";
    let expected = "\
<div>
    <?php if ($a): ?>
        <?php foreach ($items as $i): ?>
            <p><?= $i ?></p>
        <?php endforeach; ?>
    <?php endif; ?>
</div>
";
    assert_eq!(format_str(input), expected);
}

#[test]
fn cyrillic_assignment_array_splits() {
    let input = "<?php $абв = ['первыйКлюч' => 'значение один', 'второйКлюч' => 'значение два', 'третийКлюч' => 'значение три', 'четвёртыйКлюч' => 'значение четыре']; ?>";
    let expected = "\
<?php $абв = [
    'первыйКлюч' => 'значение один',
    'второйКлюч' => 'значение два',
    'третийКлюч' => 'значение три',
    'четвёртыйКлюч' => 'значение четыре',
]; ?>
";
    assert_eq!(format_str(input), expected);
}

#[test]
fn cyrillic_nested_array_fat_arrow() {
    let input = "<?php $н = ['заголовок' => 'Главная страница каталога', 'параметры' => ['ширина' => 'сто двадцать', 'высота' => 'восемьдесят пять', 'отступ' => 'десять']]; ?>";
    let expected = "\
<?php $н = [
    'заголовок' => 'Главная страница каталога',
    'параметры' => [
        'ширина' => 'сто двадцать',
        'высота' => 'восемьдесят пять',
        'отступ' => 'десять',
    ],
]; ?>
";
    assert_eq!(format_str(input), expected);
}

#[test]
fn empty_docblock_does_not_panic() {
    let input = "<?php /**/ $оченьДлинноеИмяПеременной = 'очень длинное строковое значение которое превышает лимит ширины в сто двадцать'; ?>";
    let out = format_str(input);
    assert!(out.contains("/**/"));
    assert!(out.contains("$оченьДлинноеИмяПеременной"));
}

#[test]
fn render_node_raw_preserves_php_block() {
    let mut out = String::new();
    Formatter::default().render_node_raw(&Node::PhpBlock("$x = 1;".into()), "  ", &mut out);
    assert_eq!(out, "  <?php $x = 1; ?>\n");
}

#[test]
fn render_node_raw_preserves_element_subtree() {
    let node = Node::Element {
        name: "div".into(),
        attributes: vec![Attribute {
            name: "class".into(),
            value: Some("каталог".into()),
        }],
        children: vec![Node::PhpEcho("$товар".into())],
    };
    let mut out = String::new();
    Formatter::default().render_node_raw(&node, "", &mut out);
    let expected = "\
<div class=\"каталог\">
    <?= $товар ?>
</div>
";
    assert_eq!(out, expected);
}

#[test]
fn format_never_panics_on_adversarial_input() {
    let inputs = [
        "<?php /**/ $оооооооооооооооооооооооооооооооооочень = 'длинное значение для переноса строки за лимит ширины в сто двадцать символов'; ?>",
        "<?php $ы = ['ключ' => 'значение', 'другойКлюч' => 'другое значение', 'третийКлюч' => 'третье', 'четвёртый' => 'четвёртое значение']; ?>",
        "<div><?= $переменная ?></div>",
    ];
    for input in inputs {
        let out = format_str(input);
        assert!(!out.is_empty(), "пустой вывод для: {input}");
    }
}

#[test]
fn textarea_content_preserved_verbatim() {
    let input = "<div> <textarea name=\"body\"><b>x</textarea> </div>";
    let expected = "\
<div>
    <textarea name=\"body\"><b>x</textarea>
</div>
";
    assert_eq!(format_str(input), expected);
}

#[test]
fn pre_whitespace_preserved_verbatim() {
    let input = "<pre>\n  a\n    b\n</pre>";
    let expected = "<pre>\n  a\n    b\n</pre>\n";
    assert_eq!(format_str(input), expected);
}

#[test]
fn attr_literal_double_quotes_use_single_delimiter() {
    let input = "<div data-x='{\"a\":1}'>y</div>";
    assert_eq!(format_str(input), "<div data-x='{\"a\":1}'>y</div>\n");
}

#[test]
fn attr_php_double_quotes_keep_double_delimiter() {
    let input = "<a href=\"<?= \"/u\" ?>\">x</a>";
    assert_eq!(format_str(input), "<a href=\"<?= \"/u\" ?>\">x</a>\n");
}

#[test]
fn attr_apostrophe_keeps_double_delimiter() {
    let input = "<div data-msg=\"it's ok\">y</div>";
    assert_eq!(format_str(input), "<div data-msg=\"it's ok\">y</div>\n");
}

#[test]
fn echo_block_joins_inline_run_like_short_echo() {
    let input = "<p>Цена: <?php echo $a; ?> и <?php echo $b; ?></p>";
    let expected = "<p>Цена: <?= $a ?> и <?= $b ?></p>\n";
    let once = format_str(input);
    assert_eq!(once, expected);
    assert_eq!(format_str(&once), expected);
}

#[test]
fn single_line_brace_switch_is_idempotent() {
    let input = "<?php switch ($x) { case 1: echo \"a\"; break; default: echo \"d\"; } ?>";
    let expected = "\
<?php switch ($x) {
    case 1:
        echo \"a\";
        break;
    default:
        echo \"d\";
} ?>
";
    let once = format_str(input);
    assert_eq!(once, expected);
    assert_eq!(format_str(&once), expected);
}

#[test]
fn leading_blank_lines_are_stripped() {
    assert_eq!(format_str("\n\n<p>x</p>"), "<p>x</p>\n");
}

#[test]
fn whitespace_only_input_yields_empty() {
    assert_eq!(format_str("\n\n"), "");
}

#[test]
fn closure_in_assignment_array_is_idempotent() {
    let input = "<?php $c = [\"x\" => function ($m) { return $m->status; }, \"header\" => [\"alpha\" => $m->attribute, \"beta\" => $m->other, \"gamma\" => $m->more]]; ?>";
    let once = format_str(input);
    assert_eq!(format_str(&once), once);
    assert!(once.contains("function ($m) { return $m->status; },"));
    assert!(once.lines().count() > 5);
}
