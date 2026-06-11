use super::*;
use crate::config::IndentStyle;
use crate::parser::{ast, lexer};
use pretty_assertions::assert_eq;

fn fmt(cfg: &Config, input: &str) -> String {
    let nodes = ast::parse(lexer::tokenize(input));
    Formatter::new(cfg).format(&nodes)
}

#[test]
fn indent_size_two_spaces() {
    let cfg = Config {
        indent_size: 2,
        ..Config::default()
    };
    assert_eq!(fmt(&cfg, "<div><p>x</p></div>"), "<div>\n  <p>x</p>\n</div>\n");
}

#[test]
fn indent_style_tabs() {
    let cfg = Config {
        indent_style: IndentStyle::Tabs,
        ..Config::default()
    };
    assert_eq!(fmt(&cfg, "<div><p>x</p></div>"), "<div>\n\t<p>x</p>\n</div>\n");
}

#[test]
fn smaller_line_length_splits_more() {
    let input = "<div class=\"alpha\" id=\"beta\" data-role=\"gamma\" data-extra=\"delta\">x</div>";
    let wide = fmt(&Config::default(), input);
    let narrow = fmt(
        &Config {
            max_line_length: 40,
            ..Config::default()
        },
        input,
    );
    assert!(narrow.lines().count() > wide.lines().count());
}

#[test]
fn long_or_chain_splits_by_operator() {
    let input = "<?php $access = $user->isActiveMember() || $user->isPaidSubscriber() || $user->isPlatformAdmin() || $user->isGuestInvited() || $user->isTrialUser(); ?>";
    let out = fmt(&Config::default(), input);
    assert!(out.contains("\n    || "), "expected split by ||, got:\n{out}");
    assert_eq!(out.matches("\n    || ").count(), 4, "one operand per line:\n{out}");
}

#[test]
fn mixed_precedence_keeps_and_groups_together() {
    let input = "<?php $allowed = $user->isActive() && $user->hasRole('editor') || $user->isAdmin() && $user->isOwner($model) || $user->isSuperUser(); ?>";
    let out = fmt(&Config::default(), input);
    assert!(out.contains("\n    || "), "split by ||:\n{out}");
    assert!(!out.contains("\n    && "), "&& groups must stay inline:\n{out}");
    assert!(out.contains("isActive() && $user->hasRole"), "&& kept on line:\n{out}");
}

#[test]
fn short_unwrapped_chain_collapses() {
    let input = "<?php\n$ready = $order->isPaid()\n    || $order->isFree();\n?>\n";
    let out = fmt(&Config::default(), input);
    assert_eq!(out, "<?php $ready = $order->isPaid() || $order->isFree(); ?>\n");
}

#[test]
fn operator_inside_string_does_not_trigger_split() {
    let input = "<?php $separator = $left === '||' || $right === '&&'; ?>";
    let out = fmt(&Config::default(), input);
    assert_eq!(out, "<?php $separator = $left === '||' || $right === '&&'; ?>\n");
}

#[test]
fn logical_split_is_idempotent() {
    let input = "<?php $access = $user->isActiveMember() || $user->isPaidSubscriber() || $user->isPlatformAdmin() || $user->isGuestInvited() || $user->isTrialUser(); ?>";
    let once = fmt(&Config::default(), input);
    let twice = fmt(&Config::default(), &once);
    assert_eq!(once, twice, "second pass changed output");
}

#[test]
fn body_within_tag_width_window_still_splits() {
    let input = "<?php $isVisible = $item->isEnabledNow() || $item->isHighlightedRow() || $item->isPromotedToday() || $item->isPinned(); ?>";
    let out = fmt(&Config::default(), input);
    assert!(
        out.contains("\n    || "),
        "body fits without tags but not with them, must split:\n{out}"
    );
    assert_eq!(
        out,
        fmt(&Config::default(), &out),
        "single and multiline paths must agree (idempotent)"
    );
}

#[test]
fn strips_utf8_bom() {
    let out = Formatter::default().format_source("\u{FEFF}<div>\n<span>x</span>\n</div>\n");
    assert_eq!(out, "<div>\n    <span>x</span>\n</div>\n");
}

#[test]
fn heredoc_in_echo_keeps_line_structure() {
    let src = "<div>\n<?= <<<HTML\n<b>{$user->name}</b>\nHTML ?>\n</div>\n";
    let out = Formatter::default().format_source(src);
    assert_eq!(out, "<div>\n    <?= <<<HTML\n<b>{$user->name}</b>\nHTML ?>\n</div>\n");
    assert_eq!(
        out,
        Formatter::default().format_source(&out),
        "heredoc echo must be idempotent"
    );
}

#[test]
fn conditional_comment_stays_verbatim() {
    let out = Formatter::default().format_source("<!--[if IE]><p>old</p><![endif]-->\n<div>x</div>\n");
    assert_eq!(out, "<!--[if IE]><p>old</p><![endif]-->\n<div>x</div>\n");
}

#[test]
fn svg_primitives_self_close() {
    let src = "<svg viewBox=\"0 0 24 24\"><path d=\"M5 13l4 4L19 7\"/><circle cx=\"12\" cy=\"12\" r=\"10\"/></svg>\n";
    let out = Formatter::default().format_source(src);
    assert_eq!(
        out,
        "<svg viewBox=\"0 0 24 24\">\n    <path d=\"M5 13l4 4L19 7\"/>\n    <circle cx=\"12\" cy=\"12\" r=\"10\"/>\n</svg>\n"
    );
}

#[test]
fn single_key_widget_config_splits() {
    let src = "<?= Nav::widget(['items' => [['label' => 'Home', 'url' => ['/']], ['label' => 'About Us Page Long Enough', 'url' => ['/site/about']], ['label' => 'Contact Form Long Enough', 'url' => ['/site/contact']]]]) ?>\n";
    let out = Formatter::default().format_source(src);
    assert!(out.starts_with("<?= Nav::widget([\n    'items' => [\n"), "got: {out}");
    assert!(out.lines().all(|l| l.chars().count() <= 120), "got: {out}");
}

#[test]
fn use_inside_string_is_not_header() {
    let src = "<?php $hint = \"please use the form below\"; echo $hint; ?>\n";
    let out = Formatter::default().format_source(src);
    assert!(
        !out.contains("<?php\n\n"),
        "string 'use' must not trigger header layout: {out}"
    );
}

#[test]
fn closure_use_keeps_tight_body() {
    let src = "<?php\n$fn = function ($x) use ($y) { $y += $x; return $y; };\n";
    let out = Formatter::default().format_source(src);
    assert_eq!(
        out,
        "<?php $fn = function ($x) use ($y) {\n    $y += $x;\n    return $y;\n}; ?>\n"
    );
}

#[test]
fn trait_use_list_stays_on_one_line() {
    let src = "<?php\nclass A {\nuse Hello, World {\nHello::say insteadof World;\n}\n}\n";
    let out = Formatter::default().format_source(src);
    assert!(out.contains("    use Hello, World {\n"), "got: {out}");
}

#[test]
fn closure_in_multi_statement_block_always_wraps() {
    let src = "<?php\n$total = 0;\n$add = function ($x) use (&$total) { $total += $x; return $total; };\n$add(5);\n";
    let out = Formatter::default().format_source(src);
    assert_eq!(
        out,
        "<?php $total = 0;\n$add = function ($x) use (&$total) {\n    $total += $x;\n    return $total;\n};\n$add(5); ?>\n"
    );
}

#[test]
fn braces_in_comments_do_not_shift_indentation() {
    let src = "<?php\nclass C {\npublic function a() {\n/* open { brace */\nreturn 1;\n}\npublic function b() { return 2; }\n}\n";
    let out = Formatter::default().format_source(src);
    assert_eq!(
        out,
        "<?php\n\nclass C\n{\n    public function a()\n    {\n        /* open { brace */\n        return 1;\n    }\n\n    public function b()\n    {\n        return 2;\n    }\n}\n?>\n"
    );
}
