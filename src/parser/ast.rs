use super::lexer::{Attribute, Token};

const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source", "track", "wbr",
];

fn is_void_element(name: &str) -> bool {
    VOID_ELEMENTS.contains(&name.to_lowercase().as_str())
}

#[derive(Debug, PartialEq)]
pub enum Node {
    Element {
        name: String,
        attributes: Vec<Attribute>,
        children: Vec<Node>,
    },
    Text(String),
    PhpBlock(String),
    PhpEcho(String),
}

pub fn parse(tokens: Vec<Token>) -> Vec<Node> {
    let mut stack: Vec<(String, Vec<Attribute>, Vec<Node>)> = Vec::new();
    let mut current: Vec<Node> = Vec::new();

    for token in tokens {
        match token {
            Token::OpenTag { name, attributes } => {
                if is_void_element(&name) {
                    current.push(Node::Element {
                        name,
                        attributes,
                        children: Vec::new(),
                    });
                } else {
                    stack.push((name, attributes, std::mem::take(&mut current)));
                }
            }
            Token::CloseTag(_) => {
                if let Some((name, attributes, mut parent)) = stack.pop() {
                    parent.push(Node::Element {
                        name,
                        attributes,
                        children: std::mem::take(&mut current),
                    });
                    current = parent;
                }
            }
            Token::SelfClosing { name, attributes } => {
                current.push(Node::Element {
                    name,
                    attributes,
                    children: Vec::new(),
                });
            }
            Token::Text(s) => current.push(Node::Text(s)),
            Token::PhpBlock(s) => current.push(Node::PhpBlock(s)),
            Token::PhpEcho(s) => current.push(Node::PhpEcho(s)),
        }
    }
    while let Some((name, attributes, mut parent)) = stack.pop() {
        parent.push(Node::Element {
            name,
            attributes,
            children: std::mem::take(&mut current),
        });
        current = parent;
    }

    current
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn attr(name: &str, value: Option<&str>) -> Attribute {
        Attribute {
            name: name.into(),
            value: value.map(Into::into),
        }
    }

    #[test]
    fn empty_input() {
        assert_eq!(parse(vec![]), Vec::<Node>::new());
    }

    #[test]
    fn simple_div_with_text() {
        let tokens = vec![
            Token::OpenTag {
                name: "div".into(),
                attributes: vec![],
            },
            Token::Text("hello".into()),
            Token::CloseTag("div".into()),
        ];

        assert_eq!(
            parse(tokens),
            vec![Node::Element {
                name: "div".into(),
                attributes: vec![],
                children: vec![Node::Text("hello".into())],
            }]
        );
    }

    #[test]
    fn nested_elements() {
        let tokens = vec![
            Token::OpenTag {
                name: "div".into(),
                attributes: vec![],
            },
            Token::OpenTag {
                name: "span".into(),
                attributes: vec![],
            },
            Token::Text("x".into()),
            Token::CloseTag("span".into()),
            Token::CloseTag("div".into()),
        ];

        assert_eq!(
            parse(tokens),
            vec![Node::Element {
                name: "div".into(),
                attributes: vec![],
                children: vec![Node::Element {
                    name: "span".into(),
                    attributes: vec![],
                    children: vec![Node::Text("x".into())],
                }],
            }]
        );
    }

    #[test]
    fn mixed_html_php() {
        let tokens = vec![
            Token::OpenTag {
                name: "div".into(),
                attributes: vec![attr("class", Some("item"))],
            },
            Token::PhpEcho("$name".into()),
            Token::CloseTag("div".into()),
        ];

        assert_eq!(
            parse(tokens),
            vec![Node::Element {
                name: "div".into(),
                attributes: vec![attr("class", Some("item"))],
                children: vec![Node::PhpEcho("$name".into())],
            }]
        );
    }

    #[test]
    fn php_blocks_at_top_level() {
        let tokens = vec![
            Token::PhpBlock("if ($x):".into()),
            Token::OpenTag {
                name: "p".into(),
                attributes: vec![],
            },
            Token::Text("hi".into()),
            Token::CloseTag("p".into()),
            Token::PhpBlock("endif;".into()),
        ];

        assert_eq!(
            parse(tokens),
            vec![
                Node::PhpBlock("if ($x):".into()),
                Node::Element {
                    name: "p".into(),
                    attributes: vec![],
                    children: vec![Node::Text("hi".into())],
                },
                Node::PhpBlock("endif;".into()),
            ]
        );
    }

    #[test]
    fn self_closing_in_tree() {
        let tokens = vec![
            Token::OpenTag {
                name: "div".into(),
                attributes: vec![],
            },
            Token::SelfClosing {
                name: "br".into(),
                attributes: vec![],
            },
            Token::CloseTag("div".into()),
        ];

        assert_eq!(
            parse(tokens),
            vec![Node::Element {
                name: "div".into(),
                attributes: vec![],
                children: vec![Node::Element {
                    name: "br".into(),
                    attributes: vec![],
                    children: vec![],
                }],
            }]
        );
    }
}
