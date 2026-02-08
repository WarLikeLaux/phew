use super::php::{format_php_code, join_php_lines, split_by_args, split_by_chain};
use crate::parser::ast::Node;
use crate::parser::lexer::Attribute;

const INDENT: &str = "    ";
const MAX_LINE_LENGTH: usize = 120;

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

fn count_leading_closers(s: &str) -> usize {
    s.chars().take_while(|c| matches!(c, ')' | ']' | '}')).count()
}

fn count_brackets(s: &str) -> (usize, usize) {
    let mut openers = 0usize;
    let mut closers = 0usize;
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let ch = chars[i];
        if ch == '\'' || ch == '"' {
            i += 1;
            while i < len && chars[i] != ch {
                if chars[i] == '\\' {
                    i += 1;
                }
                i += 1;
            }
        } else if matches!(ch, '(' | '[' | '{') {
            openers += 1;
        } else if matches!(ch, ')' | ']' | '}') {
            closers += 1;
        }
        i += 1;
    }

    (openers, closers)
}

fn normalize_statements(code: &str) -> String {
    let mut result = String::from("\n");
    let chars: Vec<char> = code.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let ch = chars[i];
        if ch == '\'' || ch == '"' {
            result.push(ch);
            i += 1;
            while i < len && chars[i] != ch {
                if chars[i] == '\\' {
                    result.push(chars[i]);
                    i += 1;
                }
                if i < len {
                    result.push(chars[i]);
                    i += 1;
                }
            }
            if i < len {
                result.push(chars[i]);
                i += 1;
            }
            continue;
        }
        result.push(ch);
        if ch == ';' && i + 1 < len && chars[i + 1] != '\n' {
            result.push('\n');
        }

        if ch == '/'
            && i + 1 < len
            && chars[i + 1] == '*'
            && i + 2 < len
            && chars[i + 2] == '*'
            && !result.ends_with('\n')
            && result.len() > 1
        {
            let last = result.pop().unwrap();
            result.push('\n');
            result.push(last);
        }
        if ch == '/' && result.len() >= 2 && result.ends_with("*/") {
            result.pop();
            result.pop();
            if !result.ends_with('\n') {
                let trimmed = result.trim_end().to_string();
                result.clear();
                result.push_str(&trimmed);
                result.push('\n');
            }
            result.push_str("*/\n");
            if i + 1 < len && chars[i + 1] != '\n' {
                result.push('\n');
            }
        }

        if ch == '*'
            && i + 1 < len
            && chars[i + 1] == ' '
            && i + 2 < len
            && chars[i + 2] == '@'
            && !result.ends_with('\n')
            && !result.ends_with('/')
        {
            result.pop();
            result.push('\n');
            result.push(ch);
        }
        i += 1;
    }

    result
}

fn reindent_php_block(code: &str, pad: &str) -> String {
    let code = if !code.contains('\n') && code.contains(';') {
        normalize_statements(code)
    } else {
        code.to_string()
    };
    let mut result = String::new();
    let mut depth: i32 = 0;
    let mut prev_blank = false;
    let mut first_content = true;
    let mut prev_was_doc_close = false;

    for line in code.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !prev_blank {
                result.push('\n');
                prev_blank = true;
            }
            continue;
        }

        if first_content && !prev_blank {
            result.push('\n');
        }
        first_content = false;

        if prev_was_doc_close && !prev_blank {
            result.push('\n');
        }

        prev_blank = false;
        prev_was_doc_close = trimmed == "*/";

        let formatted = format_php_code(trimmed);
        let leading = count_leading_closers(&formatted) as i32;
        let write_depth = (depth - leading).max(0) as usize;
        let inner_pad = INDENT.repeat(write_depth);

        if formatted.starts_with('*') {
            result.push_str(&format!("{pad}{inner_pad} {formatted}\n"));
        } else {
            result.push_str(&format!("{pad}{inner_pad}{formatted}\n"));
        }

        let (openers, closers) = count_brackets(&formatted);
        let net = openers as i32 - closers as i32;
        depth += net.min(1);
        depth = depth.max(0);
    }

    result
}

fn format_echo(code: &str, pad: &str) -> String {
    let joined = join_php_lines(code);
    let formatted = format_php_code(&joined);
    let single = format!("{pad}<?= {formatted} ?>");

    if single.len() <= MAX_LINE_LENGTH {
        return format!("{single}\n");
    }

    let parts = split_by_chain(&formatted);
    if parts.len() > 2 {
        let mut result = format!("{pad}<?= {}{}", parts[0], parts[1]);
        for part in &parts[2..] {
            result.push_str(&format!("\n{pad}{INDENT}{part}"));
        }
        result.push_str(" ?>");
        result.push('\n');
        return result;
    }

    if let Some((prefix, args, suffix)) = split_by_args(&formatted) {
        let mut result = format!("{pad}<?= {prefix}\n");
        for (i, arg) in args.iter().enumerate() {
            if i < args.len() - 1 {
                result.push_str(&format!("{pad}{INDENT}{arg},\n"));
            } else {
                result.push_str(&format!("{pad}{INDENT}{arg}\n"));
            }
        }
        result.push_str(&format!("{pad}{suffix} ?>\n"));
        return result;
    }

    format!("{single}\n")
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
                } else if current_depth <= 1 && s.contains('\n') && s.chars().filter(|&c| c == '\n').count() > 1 {
                    output.push('\n');
                }
            }
            Node::PhpBlock(code) => {
                let is_multiline = code.contains('\n') || code.chars().filter(|&c| c == ';').count() > 1;
                if is_multiline {
                    output.push_str(&format!("{pad}<?php\n"));
                    output.push_str(&reindent_php_block(code, &pad));
                    output.push_str(&format!("{pad}?>\n"));
                } else {
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
            }
            Node::PhpEcho(code) => {
                output.push_str(&format_echo(code, &pad));
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
