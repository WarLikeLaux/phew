use super::*;
use pretty_assertions::assert_eq;

fn open(name: &str, attrs: Vec<(&str, Option<&str>)>) -> Token {
    Token::OpenTag {
        name: name.into(),
        attributes: attrs
            .into_iter()
            .map(|(n, v)| Attribute {
                name: n.into(),
                value: v.map(Into::into),
            })
            .collect(),
    }
}

fn close(name: &str) -> Token {
    Token::CloseTag(name.into())
}

fn self_closing(name: &str, attrs: Vec<(&str, Option<&str>)>) -> Token {
    Token::SelfClosing {
        name: name.into(),
        attributes: attrs
            .into_iter()
            .map(|(n, v)| Attribute {
                name: n.into(),
                value: v.map(Into::into),
            })
            .collect(),
    }
}

fn text(s: &str) -> Token {
    Token::Text(s.into())
}

#[test]
fn empty_input() {
    assert_eq!(tokenize(""), Vec::<Token>::new());
}

#[test]
fn plain_text() {
    assert_eq!(tokenize("hello"), vec![text("hello")]);
}

#[test]
fn simple_div() {
    assert_eq!(
        tokenize("<div>hello</div>"),
        vec![open("div", vec![]), text("hello"), close("div")]
    );
}

#[test]
fn self_closing_br() {
    assert_eq!(tokenize("<br />"), vec![self_closing("br", vec![])]);
}

#[test]
fn nested_tags() {
    assert_eq!(
        tokenize("<div><span>x</span></div>"),
        vec![
            open("div", vec![]),
            open("span", vec![]),
            text("x"),
            close("span"),
            close("div"),
        ]
    );
}

#[test]
fn tag_with_class() {
    assert_eq!(
        tokenize(r#"<div class="container">hello</div>"#),
        vec![
            open("div", vec![("class", Some("container"))]),
            text("hello"),
            close("div"),
        ]
    );
}

#[test]
fn multiple_attributes() {
    assert_eq!(
        tokenize(r#"<a href="/about" class="link" id="nav">go</a>"#),
        vec![
            open(
                "a",
                vec![("href", Some("/about")), ("class", Some("link")), ("id", Some("nav")),]
            ),
            text("go"),
            close("a"),
        ]
    );
}

#[test]
fn boolean_attribute() {
    assert_eq!(
        tokenize("<input disabled />"),
        vec![self_closing("input", vec![("disabled", None)])]
    );
}

#[test]
fn single_quotes() {
    assert_eq!(
        tokenize("<div class='foo'>x</div>"),
        vec![open("div", vec![("class", Some("foo"))]), text("x"), close("div"),]
    );
}

#[test]
fn php_block() {
    assert_eq!(tokenize("<?php echo $x; ?>"), vec![Token::PhpBlock("echo $x;".into())]);
}

#[test]
fn php_echo() {
    assert_eq!(tokenize("<?= $title ?>"), vec![Token::PhpEcho("$title".into())]);
}

#[test]
fn mixed_html_php() {
    assert_eq!(
        tokenize("<div><?= $name ?></div>"),
        vec![open("div", vec![]), Token::PhpEcho("$name".into()), close("div"),]
    );
}

#[test]
fn php_with_surrounding_text() {
    assert_eq!(
        tokenize("hello <?php if ($x): ?> world"),
        vec![text("hello "), Token::PhpBlock("if ($x):".into()), text(" world"),]
    );
}

#[test]
fn short_php_tag_without_space() {
    assert_eq!(tokenize("<?if ($x): ?>"), vec![Token::PhpBlock("if ($x):".into())]);
}

#[test]
fn script_raw_text() {
    assert_eq!(
        tokenize("<script>if (a < b) { alert(1); }</script>"),
        vec![
            open("script", vec![]),
            text("if (a < b) { alert(1); }".into()),
            close("script"),
        ]
    );
}

#[test]
fn style_raw_text() {
    assert_eq!(
        tokenize("<style>.a > .b { color: red; }</style>"),
        vec![
            open("style", vec![]),
            text(".a > .b { color: red; }".into()),
            close("style"),
        ]
    );
}

#[test]
fn script_with_attributes() {
    assert_eq!(
        tokenize(r#"<script type="text/javascript">var x = 1;</script>"#),
        vec![
            open("script", vec![("type", Some("text/javascript"))]),
            text("var x = 1;".into()),
            close("script"),
        ]
    );
}

#[test]
fn textarea_raw_text() {
    assert_eq!(
        tokenize("<textarea><b>x</textarea>"),
        vec![open("textarea", vec![]), text("<b>x"), close("textarea"),]
    );
}

#[test]
fn doctype_token() {
    assert_eq!(tokenize("<!DOCTYPE html>"), vec![Token::Doctype("html".into())]);
}

#[test]
fn comment_token() {
    assert_eq!(
        tokenize("<!-- This is a comment -->"),
        vec![Token::Comment("This is a comment".into())]
    );
}

#[test]
fn doctype_and_comment_with_html() {
    assert_eq!(
        tokenize("<!DOCTYPE html>\n<html>\n<!-- comment -->\n<body></body>\n</html>"),
        vec![
            Token::Doctype("html".into()),
            text("\n"),
            open("html", vec![]),
            text("\n"),
            Token::Comment("comment".into()),
            text("\n"),
            open("body", vec![]),
            close("body"),
            text("\n"),
            close("html"),
        ]
    );
}
