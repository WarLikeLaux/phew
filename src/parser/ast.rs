use super::lexer::{Attribute, Token};

const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source", "track", "wbr",
];

fn is_void_element(name: &str) -> bool {
    VOID_ELEMENTS.contains(&name.to_lowercase().as_str())
}

const FOREIGN_ROOTS: &[&str] = &["svg", "math"];

fn in_foreign_content(stack: &[(String, Vec<Attribute>, Vec<Node>)]) -> bool {
    stack
        .iter()
        .any(|(n, _, _)| FOREIGN_ROOTS.contains(&n.to_lowercase().as_str()))
}

#[derive(Debug, PartialEq)]
pub enum Node {
    Element {
        name: String,
        attributes: Vec<Attribute>,
        children: Vec<Node>,
        foreign: bool,
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
                    foreign: in_foreign_content(stack),
                });
                *current = parent;
            }
        }
        if let Some((name, attributes, mut parent)) = stack.pop() {
            parent.push(Node::Element {
                name,
                attributes,
                children: std::mem::take(current),
                foreign: in_foreign_content(stack),
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
                        foreign: in_foreign_content(&stack),
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
                    foreign: in_foreign_content(&stack),
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
            foreign: in_foreign_content(&stack),
        });
        current = parent;
    }

    current
}

#[cfg(test)]
#[path = "ast_tests.rs"]
mod tests;
