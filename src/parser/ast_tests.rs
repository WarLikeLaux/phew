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
            foreign: false,
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
                foreign: false,
            }],
            foreign: false,
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
            foreign: false,
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
                foreign: false,
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
                foreign: false,
            }],
            foreign: false,
        }]
    );
}
