use super::php::format_php_code;
use crate::parser::ast::Node;
use crate::parser::lexer::Attribute;

const INDENT: &str = "    ";

const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source", "track", "wbr",
];

fn is_php_block_opener(code: &str) -> bool {
    code.trim().ends_with(':')
}

fn is_php_block_closer(code: &str) -> bool {
    let lower = code.trim().to_lowercase();
    lower.starts_with("endif")
        || lower.starts_with("endforeach")
        || lower.starts_with("endfor")
        || lower.starts_with("endwhile")
        || lower.starts_with("else")
        || lower.starts_with("elseif")
}

fn is_void_element(name: &str) -> bool {
    VOID_ELEMENTS.contains(&name.to_lowercase().as_str())
}

fn format_attributes(attrs: &[Attribute]) -> String {
    if attrs.is_empty() {
        return String::new();
    }

    let parts: Vec<String> = attrs
        .iter()
        .map(|a| match &a.value {
            Some(v) => format!("{}=\"{}\"", a.name, v),
            None => a.name.clone(),
        })
        .collect();

    format!(" {}", parts.join(" "))
}

fn is_inline_content(children: &[Node]) -> bool {
    children.len() <= 2 && children.iter().all(|c| matches!(c, Node::Text(_) | Node::PhpEcho(_)))
}

fn format_inline(name: &str, attributes: &[Attribute], children: &[Node]) -> String {
    let attrs = format_attributes(attributes);
    let content: String = children
        .iter()
        .map(|c| match c {
            Node::Text(s) => s.trim().to_string(),
            Node::PhpEcho(s) => format!("<?= {} ?>", format_php_code(s)),
            _ => String::new(),
        })
        .collect();
    format!("<{name}{attrs}>{content}</{name}>")
}

fn format_nodes(nodes: &[Node], depth: usize, output: &mut String) {
    let mut current_depth = depth;

    for node in nodes {
        let pad = INDENT.repeat(current_depth);

        match node {
            Node::Element {
                name,
                attributes,
                children,
            } => {
                let attrs = format_attributes(attributes);

                if children.is_empty() && is_void_element(name) {
                    output.push_str(&format!("{pad}<{name}{attrs} />\n"));
                } else if is_inline_content(children) {
                    output.push_str(&pad);
                    output.push_str(&format_inline(name, attributes, children));
                    output.push('\n');
                } else {
                    output.push_str(&format!("{pad}<{name}{attrs}>\n"));
                    format_nodes(children, current_depth + 1, output);
                    output.push_str(&format!("{pad}</{name}>\n"));
                }
            }
            Node::Text(s) => {
                let trimmed = s.trim();
                if !trimmed.is_empty() {
                    output.push_str(&format!("{pad}{trimmed}\n"));
                }
            }
            Node::PhpBlock(code) => {
                let formatted = format_php_code(code);
                if is_php_block_closer(code) {
                    current_depth = current_depth.saturating_sub(1);
                    let pad_less = INDENT.repeat(current_depth);
                    output.push_str(&format!("{pad_less}<?php {formatted} ?>\n"));
                } else {
                    output.push_str(&format!("{pad}<?php {formatted} ?>\n"));
                    if is_php_block_opener(code) {
                        current_depth += 1;
                    }
                }
            }
            Node::PhpEcho(code) => {
                let formatted = format_php_code(code);
                output.push_str(&format!("{pad}<?= {formatted} ?>\n"));
            }
        }
    }
}

pub fn format(nodes: &[Node]) -> String {
    let mut output = String::new();
    format_nodes(nodes, 0, &mut output);
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{ast, lexer};
    use pretty_assertions::assert_eq;

    fn format_str(input: &str) -> String {
        let tokens = lexer::tokenize(input);
        let nodes = ast::parse(tokens);
        format(&nodes)
    }

    #[test]
    fn simple_div() {
        assert_eq!(format_str("<div>hello</div>"), "<div>hello</div>\n");
    }

    #[test]
    fn nested_html() {
        let input = "<div><p>text</p></div>";
        let expected = "\
<div>
    <p>text</p>
</div>
";
        assert_eq!(format_str(input), expected);
    }

    #[test]
    fn self_closing_tag() {
        assert_eq!(format_str("<br />"), "<br />\n");
    }

    #[test]
    fn php_echo_inline() {
        let input = "<h1><?= $title ?></h1>";
        assert_eq!(format_str(input), "<h1><?= $title ?></h1>\n");
    }

    #[test]
    fn php_block_indentation() {
        let input = "<div><?php if ($x): ?><p>yes</p><?php endif; ?></div>";
        let expected = "\
<div>
    <?php if ($x): ?>
        <p>yes</p>
    <?php endif; ?>
</div>
";
        assert_eq!(format_str(input), expected);
    }

    #[test]
    fn attributes_preserved() {
        let input = r#"<div class="container" id="main"><p>hi</p></div>"#;
        let expected = "\
<div class=\"container\" id=\"main\">
    <p>hi</p>
</div>
";
        assert_eq!(format_str(input), expected);
    }

    #[test]
    fn nested_php_blocks() {
        let input = "<div><?php if ($a): ?><?php foreach ($items as $i): ?><p><?= $i ?></p><?php endforeach; ?><?php endif; ?></div>";
        let expected = "\
<div>
    <?php if ($a): ?>
        <?php foreach ($items as $i): ?>
            <p><?= $i ?></p>
        <?php endforeach; ?>
    <?php endif; ?>
</div>
";
        assert_eq!(format_str(input), expected);
    }
}
