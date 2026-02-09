use std::iter::Peekable;

#[derive(Debug, PartialEq)]
pub struct Attribute {
    pub name: String,
    pub value: Option<String>,
}

const RAW_TEXT_ELEMENTS: &[&str] = &["script", "style", "textarea"];

#[derive(Debug, PartialEq)]
pub enum Token {
    Text(String),
    OpenTag { name: String, attributes: Vec<Attribute> },
    CloseTag(String),
    SelfClosing { name: String, attributes: Vec<Attribute> },
    PhpBlock(String),
    PhpEcho(String),
    Doctype(String),
    Comment(String),
}

fn skip_whitespace(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    while let Some(&c) = chars.peek() {
        if !c.is_whitespace() {
            break;
        }
        chars.next();
    }
}

fn consume_attr_name(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> String {
    let mut name = String::new();
    while let Some(&c) = chars.peek() {
        if c == '=' || c.is_whitespace() {
            break;
        }
        name.push(c);
        chars.next();
    }
    name
}

fn consume_attr_value(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> String {
    let mut value = String::new();

    if let Some(&quote) = chars.peek()
        && (quote == '"' || quote == '\'')
    {
        chars.next();
        while let Some(&c) = chars.peek() {
            if c == quote {
                chars.next();
                break;
            }
            value.push(c);
            chars.next();
        }
        return value;
    }

    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            break;
        }
        value.push(c);
        chars.next();
    }

    value
}

fn try_consume_php_attr(chars: &mut Peekable<std::str::Chars<'_>>) -> Option<Attribute> {
    if chars.peek() != Some(&'<') {
        return None;
    }
    let mut lookahead = chars.clone();
    lookahead.next();
    if lookahead.peek() != Some(&'?') {
        return None;
    }
    let mut php_buf = String::from("<?");
    chars.next();
    chars.next();
    while let Some(&c) = chars.peek() {
        php_buf.push(c);
        chars.next();
        if c == '?' && chars.peek() == Some(&'>') {
            php_buf.push('>');
            chars.next();
            break;
        }
    }
    Some(Attribute {
        name: php_buf,
        value: None,
    })
}

fn parse_attributes(raw: &str) -> Vec<Attribute> {
    let mut attrs = Vec::new();
    let mut chars = raw.chars().peekable();

    loop {
        skip_whitespace(&mut chars);

        if let Some(attr) = try_consume_php_attr(&mut chars) {
            attrs.push(attr);
            continue;
        }

        let name = consume_attr_name(&mut chars);
        if name.is_empty() {
            break;
        }

        skip_whitespace(&mut chars);

        if chars.peek() == Some(&'=') {
            chars.next();
            skip_whitespace(&mut chars);
            let value = consume_attr_value(&mut chars);
            attrs.push(Attribute {
                name,
                value: Some(value),
            });
        } else {
            attrs.push(Attribute { name, value: None });
        }
    }

    attrs
}

fn parse_tag(tag_content: &str) -> Token {
    let trimmed = tag_content.trim();

    if let Some(name) = trimmed.strip_prefix('/') {
        return Token::CloseTag(name.trim().to_string());
    }

    let is_self_closing = trimmed.ends_with('/');
    let body = if is_self_closing {
        trimmed[..trimmed.len() - 1].trim()
    } else {
        trimmed
    };

    let (name, rest) = match body.find(|c: char| c.is_whitespace()) {
        Some(pos) => (&body[..pos], body[pos..].trim_start()),
        None => (body, ""),
    };

    let attributes = parse_attributes(rest);

    if is_self_closing {
        Token::SelfClosing {
            name: name.to_string(),
            attributes,
        }
    } else {
        Token::OpenTag {
            name: name.to_string(),
            attributes,
        }
    }
}

fn consume_php_block(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> String {
    let mut content = String::new();
    let mut in_string: Option<char> = None;
    while let Some(&c) = chars.peek() {
        if let Some(q) = in_string {
            content.push(c);
            chars.next();
            if c == '\\' {
                if let Some(&esc) = chars.peek() {
                    content.push(esc);
                    chars.next();
                }
            } else if c == q {
                in_string = None;
            }
            continue;
        }
        if c == '\'' || c == '"' {
            in_string = Some(c);
            content.push(c);
            chars.next();
            continue;
        }
        if c == '?' {
            chars.next();
            if chars.peek() == Some(&'>') {
                chars.next();
                break;
            }
            content.push('?');
            continue;
        }
        content.push(c);
        chars.next();
    }
    let trimmed = content.trim_end().to_string();
    if trimmed.contains('\n') {
        trimmed.strip_prefix('\n').unwrap_or(&trimmed).to_string()
    } else {
        trimmed.trim().to_string()
    }
}

fn consume_raw_text(chars: &mut std::iter::Peekable<std::str::Chars<'_>>, tag_name: &str) -> (String, bool) {
    let mut content = String::new();
    let close_pattern = format!("</{}", tag_name);
    let close_upper = close_pattern.to_uppercase();

    while chars.peek().is_some() {
        let rest: String = chars.clone().collect();
        let rest_upper = rest.to_uppercase();
        if rest_upper.starts_with(&close_upper) {
            for _ in 0..close_pattern.len() {
                chars.next();
            }
            while let Some(&c) = chars.peek() {
                chars.next();
                if c == '>' {
                    break;
                }
            }
            return (content, true);
        }
        if let Some(c) = chars.next() {
            content.push(c);
        }
    }

    (content, false)
}

fn consume_php_tag_prefix(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> bool {
    if !matches!(chars.peek(), Some(&'h') | Some(&'H')) {
        return false;
    }
    chars.next();
    if !matches!(chars.peek(), Some(&'p') | Some(&'P')) {
        return false;
    }
    chars.next();
    true
}

fn try_consume_php(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Option<Token> {
    let mut look = chars.clone();

    if look.next() != Some('?') {
        return None;
    }

    match look.peek() {
        Some(&'=') => {
            look.next();
            chars.next();
            chars.next();
            let content = consume_php_block(chars);
            Some(Token::PhpEcho(content))
        }
        Some(&'p') | Some(&'P') => {
            look.next();
            if !consume_php_tag_prefix(&mut look) {
                return None;
            }
            chars.next();
            chars.next();
            consume_php_tag_prefix(chars);
            let content = consume_php_block(chars);
            Some(Token::PhpBlock(content))
        }
        Some(&' ') | Some(&'\n') | Some(&'\r') | Some(&'\t') => {
            chars.next();
            let content = consume_php_block(chars);
            Some(Token::PhpBlock(content))
        }
        Some(_) => {
            chars.next();
            let content = consume_php_block(chars);
            Some(Token::PhpBlock(content))
        }
        _ => None,
    }
}

fn try_consume_comment(chars: &mut Peekable<std::str::Chars<'_>>) -> Option<Token> {
    let mut look = chars.clone();
    look.next();
    let next_two: String = look.take(2).collect();
    if next_two != "--" {
        return None;
    }
    chars.next();
    chars.next();
    chars.next();
    let mut comment = String::new();
    loop {
        match chars.next() {
            None => break,
            Some('-') => {
                if chars.peek() == Some(&'-') {
                    chars.next();
                    if chars.peek() == Some(&'>') {
                        chars.next();
                        break;
                    }
                    comment.push('-');
                    comment.push('-');
                } else {
                    comment.push('-');
                }
            }
            Some(c) => comment.push(c),
        }
    }
    Some(Token::Comment(comment.trim().to_string()))
}

fn try_consume_doctype(chars: &mut Peekable<std::str::Chars<'_>>) -> Option<Token> {
    let mut look = chars.clone();
    look.next();
    let rest: String = look.take(7).collect();
    if !rest.to_uppercase().starts_with("DOCTYPE") {
        return None;
    }
    chars.next();
    for _ in 0..7 {
        chars.next();
    }
    let mut buf = String::new();
    while let Some(&c) = chars.peek() {
        chars.next();
        if c == '>' {
            break;
        }
        buf.push(c);
    }
    Some(Token::Doctype(buf.trim().to_string()))
}

fn consume_php_in_tag(chars: &mut Peekable<std::str::Chars<'_>>, buf: &mut String) {
    buf.push('<');
    buf.push('?');
    chars.next();
    while let Some(&pc) = chars.peek() {
        buf.push(pc);
        chars.next();
        if pc == '?' && chars.peek() == Some(&'>') {
            buf.push('>');
            chars.next();
            break;
        }
    }
}

fn consume_tag_body(chars: &mut Peekable<std::str::Chars<'_>>) -> String {
    let mut buf = String::new();
    let mut in_quote: Option<char> = None;
    while let Some(&c) = chars.peek() {
        if let Some(q) = in_quote {
            if c == '<' {
                chars.next();
                if chars.peek() == Some(&'?') {
                    consume_php_in_tag(chars, &mut buf);
                } else {
                    buf.push('<');
                }
            } else {
                buf.push(c);
                chars.next();
                if c == q {
                    in_quote = None;
                }
            }
        } else if c == '<' {
            chars.next();
            if chars.peek() == Some(&'?') {
                consume_php_in_tag(chars, &mut buf);
            } else {
                buf.push('<');
            }
        } else if c == '"' || c == '\'' {
            in_quote = Some(c);
            buf.push(c);
            chars.next();
        } else if c == '>' {
            chars.next();
            break;
        } else {
            buf.push(c);
            chars.next();
        }
    }
    buf
}

fn emit_tag_token(tag_buf: &str, chars: &mut Peekable<std::str::Chars<'_>>, tokens: &mut Vec<Token>) {
    let tag = parse_tag(tag_buf);
    if let Token::OpenTag { ref name, .. } = tag {
        if RAW_TEXT_ELEMENTS.contains(&name.to_lowercase().as_str()) {
            let tag_name = name.clone();
            tokens.push(tag);
            let (raw_content, found_close) = consume_raw_text(chars, &tag_name);
            if !raw_content.is_empty() {
                tokens.push(Token::Text(raw_content));
            }
            if found_close {
                tokens.push(Token::CloseTag(tag_name));
            }
            return;
        }
    }
    tokens.push(tag);
}

pub fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    let mut text_buf = String::new();

    while let Some(&ch) = chars.peek() {
        if ch == '<' {
            chars.next();

            if let Some(php_token) = try_consume_php(&mut chars) {
                if !text_buf.is_empty() {
                    tokens.push(Token::Text(std::mem::take(&mut text_buf)));
                }
                tokens.push(php_token);
                continue;
            }

            if !text_buf.is_empty() {
                tokens.push(Token::Text(std::mem::take(&mut text_buf)));
            }

            if chars.peek() == Some(&'!') {
                if let Some(t) = try_consume_comment(&mut chars) {
                    tokens.push(t);
                    continue;
                }
                if let Some(t) = try_consume_doctype(&mut chars) {
                    tokens.push(t);
                    continue;
                }
            }

            let tag_buf = consume_tag_body(&mut chars);
            emit_tag_token(&tag_buf, &mut chars, &mut tokens);
        } else {
            text_buf.push(ch);
            chars.next();
        }
    }

    if !text_buf.is_empty() {
        tokens.push(Token::Text(text_buf));
    }

    tokens
}

#[cfg(test)]
mod tests {
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
}
