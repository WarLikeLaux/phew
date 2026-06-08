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
