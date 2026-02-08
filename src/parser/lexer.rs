#[derive(Debug, PartialEq)]
pub struct Attribute {
    pub name: String,
    pub value: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum Token {
    Text(String),
    OpenTag { name: String, attributes: Vec<Attribute> },
    CloseTag(String),
    SelfClosing { name: String, attributes: Vec<Attribute> },
    PhpBlock(String),
    PhpEcho(String),
}

fn parse_attributes(raw: &str) -> Vec<Attribute> {
    let mut attrs = Vec::new();
    let mut chars = raw.chars().peekable();

    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace() {
            chars.next();
            continue;
        }

        let mut name = String::new();
        while let Some(&c) = chars.peek() {
            if c == '=' || c.is_whitespace() {
                break;
            }
            name.push(c);
            chars.next();
        }

        if name.is_empty() {
            break;
        }

        while let Some(&c) = chars.peek() {
            if !c.is_whitespace() {
                break;
            }
            chars.next();
        }

        if chars.peek() == Some(&'=') {
            chars.next();

            while let Some(&c) = chars.peek() {
                if !c.is_whitespace() {
                    break;
                }
                chars.next();
            }

            let mut value = String::new();

            if let Some(&quote) = chars.peek() {
                if quote == '"' || quote == '\'' {
                    chars.next();
                    while let Some(&c) = chars.peek() {
                        if c == quote {
                            chars.next();
                            break;
                        }
                        value.push(c);
                        chars.next();
                    }
                } else {
                    while let Some(&c) = chars.peek() {
                        if c.is_whitespace() {
                            break;
                        }
                        value.push(c);
                        chars.next();
                    }
                }
            }

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
    while let Some(&c) = chars.peek() {
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
    content.trim().to_string()
}

fn try_consume_php(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Option<Token> {
    if chars.peek() != Some(&'?') {
        return None;
    }
    chars.next();

    match chars.peek() {
        Some(&'=') => {
            chars.next();
            let content = consume_php_block(chars);
            Some(Token::PhpEcho(content))
        }
        Some(&'p') => {
            chars.next();
            if chars.peek() == Some(&'h') {
                chars.next();
                if chars.peek() == Some(&'p') {
                    chars.next();
                    let content = consume_php_block(chars);
                    Some(Token::PhpBlock(content))
                } else {
                    None
                }
            } else {
                None
            }
        }
        _ => None,
    }
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

            let mut tag_buf = String::new();

            while let Some(&c) = chars.peek() {
                if c == '>' {
                    chars.next();
                    break;
                }
                tag_buf.push(c);
                chars.next();
            }

            tokens.push(parse_tag(&tag_buf));
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
}
