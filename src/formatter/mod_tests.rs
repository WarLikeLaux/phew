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
