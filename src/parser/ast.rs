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
    Doctype(String),
    Comment(String),
}

fn close_tag_unwind(close_name: &str, stack: &mut Vec<(String, Vec<Attribute>, Vec<Node>)>, current: &mut Vec<Node>) {
    let close_lower = close_name.to_lowercase();
    if let Some(pos) = stack.iter().rposition(|(n, _, _)| n.to_lowercase() == close_lower) {
        while stack.len() > pos + 1 {
            if let Some((name, attributes, mut parent)) = stack.pop() {
                parent.push(Node::Element {
                    name,
                    attributes,
                    children: std::mem::take(current),
                });
                *current = parent;
            }
        }
        if let Some((name, attributes, mut parent)) = stack.pop() {
            parent.push(Node::Element {
                name,
                attributes,
                children: std::mem::take(current),
            });
            *current = parent;
        }
    }
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
            Token::CloseTag(close_name) => close_tag_unwind(&close_name, &mut stack, &mut current),
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
            Token::Doctype(s) => current.push(Node::Doctype(s)),
            Token::Comment(s) => current.push(Node::Comment(s)),
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
#[path = "ast_tests.rs"]
mod tests;
