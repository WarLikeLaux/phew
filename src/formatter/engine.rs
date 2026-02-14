use super::docblock::{emit_docblock_php, expand_single_line_docblock, is_docblock_only};
use super::echo::{contains_break, format_echo, is_echo_block_closer, is_echo_block_opener, is_single_echo_block};
use super::indent::{
    INDENT, MAX_LINE_LENGTH, count_semicolons_outside_parens, has_switch_case, is_header_php_block,
    is_php_block_closer, is_php_block_opener, is_switch_case_peer, reindent_php_block, split_header_and_opener,
};
use super::php::format_php_code;
use super::split::find_ternary_positions;
use crate::parser::ast::Node;
use crate::parser::lexer::Attribute;

const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source", "track", "wbr",
];

const RAW_TEXT_ELEMENTS: &[&str] = &["script", "style", "textarea"];

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

fn format_attribute(attr: &Attribute) -> String {
    match &attr.value {
        Some(v) => format!("{}=\"{}\"", attr.name, v),
        None => attr.name.clone(),
    }
}

struct TagParams<'a> {
    name: &'a str,
    attributes: &'a [Attribute],
    self_closing: bool,
}

fn emit_open_tag(tag: &TagParams, pad: &str, output: &mut String) {
    let attrs = format_attributes(tag.attributes);
    let tail = if tag.self_closing { " />" } else { ">" };
    let name = tag.name;
    let single = format!("{pad}<{name}{attrs}{tail}");

    if tag.attributes.is_empty() || single.len() <= MAX_LINE_LENGTH {
        output.push_str(&single);
        output.push('\n');
        return;
    }

    output.push_str(&format!("{pad}<{name}\n"));
    let attr_pad = format!("{pad}{INDENT}");
    for attr in tag.attributes {
        output.push_str(&attr_pad);
        output.push_str(&format_attribute(attr));
        output.push('\n');
    }
    output.push_str(&format!("{pad}{tail}\n"));
}

fn is_inline_content(children: &[Node]) -> bool {
    children.iter().all(|c| match c {
        Node::Text(_) | Node::PhpEcho(_) => true,
        Node::PhpBlock(code) => is_single_echo_block(code),
        _ => false,
    })
}

fn format_inline(name: &str, attributes: &[Attribute], children: &[Node]) -> String {
    let attrs = format_attributes(attributes);
    let content: String = children
        .iter()
        .map(|c| match c {
            Node::Text(s) => s.trim().to_string(),
            Node::PhpEcho(s) => format!("<?= {} ?>", format_php_code(s)),
            Node::PhpBlock(s) if is_single_echo_block(s) => {
                let expr = s.trim().strip_prefix("echo ").unwrap_or(s);
                let expr = expr.strip_suffix(';').unwrap_or(expr).trim();
                format!("<?= {} ?>", format_php_code(expr))
            }
            _ => String::new(),
        })
        .collect();
    format!("<{name}{attrs}>{content}</{name}>")
}

fn emit_element(name: &str, attributes: &[Attribute], children: &[Node], ctx: (usize, &mut String)) {
    let (depth, output) = ctx;
    let pad = INDENT.repeat(depth);
    if RAW_TEXT_ELEMENTS.contains(&name.to_lowercase().as_str()) {
        emit_open_tag(
            &TagParams {
                name,
                attributes,
                self_closing: false,
            },
            &pad,
            output,
        );
        for child in children {
            if let Node::Text(s) = child {
                let trimmed = s.trim_start_matches('\n').trim_end();
                if !trimmed.is_empty() {
                    for line in trimmed.lines() {
                        if line.chars().next().is_some_and(char::is_whitespace) {
                            output.push_str(line);
                        } else {
                            output.push_str(&pad);
                            output.push_str(line);
                        }
                        output.push('\n');
                    }
                }
            }
        }
        output.push_str(&format!("{pad}</{name}>\n"));
    } else if children.is_empty() && is_void_element(name) {
        emit_open_tag(
            &TagParams {
                name,
                attributes,
                self_closing: true,
            },
            &pad,
            output,
        );
    } else if is_inline_content(children) {
        let inline = format_inline(name, attributes, children);
        if pad.len() + inline.len() <= MAX_LINE_LENGTH {
            output.push_str(&pad);
            output.push_str(&inline);
            output.push('\n');
        } else {
            emit_open_tag(
                &TagParams {
                    name,
                    attributes,
                    self_closing: false,
                },
                &pad,
                output,
            );
            format_nodes(children, depth + 1, output);
            output.push_str(&format!("{pad}</{name}>\n"));
        }
    } else {
        emit_open_tag(
            &TagParams {
                name,
                attributes,
                self_closing: false,
            },
            &pad,
            output,
        );
        format_nodes(children, depth + 1, output);
        output.push_str(&format!("{pad}</{name}>\n"));
    }
}

struct PhpDepthState {
    depth: usize,
    switch_stack: Vec<usize>,
}

fn emit_switch_stmt(trimmed: &str, state: &mut PhpDepthState, output: &mut String) {
    let formatted = format_php_code(trimmed);
    let lower = trimmed.to_lowercase();
    if lower.starts_with("switch") && is_php_block_opener(trimmed) {
        let pad = INDENT.repeat(state.depth);
        output.push_str(&format!("{pad}<?php {formatted} ?>\n"));
        state.switch_stack.push(state.depth);
        state.depth += 1;
    } else if is_switch_case_peer(trimmed) {
        let lvl = state
            .switch_stack
            .last()
            .copied()
            .unwrap_or(state.depth.saturating_sub(1));
        let case_lvl = lvl + 1;
        let pad = INDENT.repeat(case_lvl);
        output.push_str(&format!("{pad}<?php {formatted} ?>\n"));
        state.depth = case_lvl + 1;
    } else if lower.starts_with("endswitch") {
        let lvl = state.switch_stack.pop().unwrap_or(state.depth.saturating_sub(1));
        let pad = INDENT.repeat(lvl);
        output.push_str(&format!("{pad}<?php {formatted} ?>\n"));
        state.depth = lvl;
    } else if contains_break(&lower) {
        let pad = INDENT.repeat(state.depth);
        output.push_str(&format!("{pad}<?php {formatted} ?>\n"));
    } else {
        let pad = INDENT.repeat(state.depth);
        output.push_str(&format!("{pad}<?php {formatted} ?>\n"));
        if is_php_block_opener(trimmed) {
            state.depth += 1;
        }
    }
}

fn emit_multiline_php(code: &str, pad: &str, depth: &mut usize, output: &mut String) {
    let is_header = is_header_php_block(code);
    if is_header {
        if let Some((header_code, opener_line)) = split_header_and_opener(code) {
            output.push_str(&format!("{pad}<?php\n"));
            let reindented = reindent_php_block(&header_code, pad);
            output.push_str(&reindented);
            output.push('\n');
            output.push_str(&format!("{pad}?>\n"));
            let formatted = format_php_code(&opener_line);
            output.push_str(&format!("{pad}<?php {formatted} ?>\n"));
            *depth += 1;
            return;
        }
        output.push_str(&format!("{pad}<?php\n"));
        let reindented = reindent_php_block(code, pad);
        output.push_str(&reindented);
        output.push('\n');
        output.push_str(&format!("{pad}?>\n"));
    } else {
        emit_multiline_php_inline(code, pad, output);
    }
    let has_widget_pair = code.contains("::begin(") || code.contains("::end(");
    if has_widget_pair || !is_header {
        if is_php_block_closer(code) && has_widget_pair {
            *depth = depth.saturating_sub(1);
        } else if (has_widget_pair || !is_php_block_closer(code)) && is_php_block_opener(code) {
            *depth += 1;
        }
    }
}

fn emit_multiline_php_inline(code: &str, pad: &str, output: &mut String) {
    let reindented = reindent_php_block(code, pad);
    let lines: Vec<&str> = reindented.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.len() > 1 {
        output.push_str(&format!("{pad}<?php {}\n", lines[0].trim_start()));
        for line in &lines[1..lines.len() - 1] {
            output.push_str(line);
            output.push('\n');
        }
        output.push_str(&format!("{} ?>\n", lines[lines.len() - 1]));
    } else if lines.len() == 1 {
        output.push_str(&format!("{pad}<?php {} ?>\n", lines[0].trim_start()));
    }
}

fn emit_single_php(code: &str, pad: &str, state: &mut PhpDepthState, output: &mut String) {
    let formatted = format_php_code(code);
    let lower = code.trim().to_lowercase();
    if lower.starts_with("switch") && is_php_block_opener(code) && !is_php_block_closer(code) {
        let stmt_pad = INDENT.repeat(state.depth);
        output.push_str(&format!("{stmt_pad}<?php {formatted} ?>\n"));
        state.switch_stack.push(state.depth);
        state.depth += 1;
    } else if !state.switch_stack.is_empty() && is_switch_case_peer(code) {
        let lvl = state
            .switch_stack
            .last()
            .copied()
            .unwrap_or(state.depth.saturating_sub(1));
        let case_lvl = lvl + 1;
        let stmt_pad = INDENT.repeat(case_lvl);
        output.push_str(&format!("{stmt_pad}<?php {formatted} ?>\n"));
        state.depth = case_lvl + 1;
    } else if !state.switch_stack.is_empty() && lower.starts_with("endswitch") {
        let lvl = state.switch_stack.pop().unwrap_or(state.depth.saturating_sub(1));
        let stmt_pad = INDENT.repeat(lvl);
        output.push_str(&format!("{stmt_pad}<?php {formatted} ?>\n"));
        state.depth = lvl;
    } else if !state.switch_stack.is_empty() && contains_break(&lower) {
        let stmt_pad = INDENT.repeat(state.depth);
        output.push_str(&format!("{stmt_pad}<?php {formatted} ?>\n"));
    } else if is_php_block_closer(code) {
        state.depth = state.depth.saturating_sub(1);
        let pad_less = INDENT.repeat(state.depth);
        output.push_str(&format!("{pad_less}<?php {formatted} ?>\n"));
        if is_php_block_opener(code) {
            state.depth += 1;
        }
    } else {
        emit_single_php_long(code, pad, &mut state.depth, output);
    }
}

fn emit_single_php_long(code: &str, pad: &str, depth: &mut usize, output: &mut String) {
    let formatted = format_php_code(code);
    if is_header_php_block(code) {
        output.push_str(&format!("{pad}<?php\n"));
        let reindented = reindent_php_block(code, pad);
        output.push_str(&reindented);
        output.push('\n');
        output.push_str(&format!("{pad}?>\n"));
        return;
    }
    if let Some(docblock) = expand_single_line_docblock(code) {
        emit_docblock_php(&docblock, pad, output);
        return;
    }
    let single = format!("{pad}<?php {formatted} ?>");
    if single.len() <= MAX_LINE_LENGTH {
        output.push_str(&format!("{single}\n"));
    } else if let Some((q_pos, c_pos)) = find_ternary_positions(&formatted) {
        let condition = formatted[..q_pos].trim_end();
        let true_val = formatted[q_pos + 1..c_pos].trim();
        let false_val = formatted[c_pos + 1..].trim();
        let inner_pad = format!("{pad}{INDENT}");
        output.push_str(&format!(
            "{pad}<?php {condition}\n{inner_pad}? {true_val}\n{inner_pad}: {false_val} ?>\n"
        ));
    } else {
        let reindented = reindent_php_block(code, pad);
        let lines: Vec<&str> = reindented.lines().filter(|l| !l.trim().is_empty()).collect();
        if lines.len() > 1 {
            output.push_str(&format!("{pad}<?php {}\n", lines[0].trim_start()));
            for line in &lines[1..lines.len() - 1] {
                output.push_str(line);
                output.push('\n');
            }
            output.push_str(&format!("{} ?>\n", lines[lines.len() - 1]));
        } else {
            output.push_str(&format!("{pad}<?php\n"));
            output.push_str(&reindented);
            output.push_str(&format!("{pad}?>\n"));
        }
    }
    if is_php_block_opener(code) {
        *depth += 1;
    }
}

fn emit_php_block(code: &str, pad: &str, state: &mut PhpDepthState, output: &mut String) {
    let trimmed = code.trim();
    if let Some(expr) = trimmed.strip_prefix("echo ") {
        let expr = expr.strip_suffix(';').unwrap_or(expr).trim();
        let semicolons = count_semicolons_outside_parens(code);
        if semicolons <= 1 && !expr.contains('\n') {
            emit_php_echo(expr, pad, state, output);
            return;
        }
    }
    if is_docblock_only(code) {
        emit_docblock_php(code, pad, output);
        return;
    }
    let semicolons = count_semicolons_outside_parens(code);
    let is_multiline = code.contains('\n') || semicolons > 1 || has_switch_case(code);
    if is_multiline && has_switch_case(code) {
        emit_php_switch_block(code, state, output);
    } else if is_multiline {
        emit_multiline_php(code, pad, &mut state.depth, output);
    } else {
        emit_single_php(code, pad, state, output);
    }
}

fn emit_php_switch_block(code: &str, state: &mut PhpDepthState, output: &mut String) {
    let normalized = super::indent::normalize_statements(code);
    let statements: Vec<&str> = normalized
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    let mut i = 0usize;
    while i < statements.len() {
        let current = statements[i];
        if current.to_lowercase().starts_with("switch")
            && is_php_block_opener(current)
            && i + 1 < statements.len()
            && is_switch_case_peer(statements[i + 1])
        {
            let switch_depth = state.depth;
            let stmt_pad = INDENT.repeat(switch_depth);
            let switch_stmt = format_php_code(current);
            let first_case = format_php_code(statements[i + 1]);
            let case_pad = format!("{stmt_pad}{INDENT}");
            output.push_str(&format!("{stmt_pad}<?php {switch_stmt}\n{case_pad}{first_case} ?>\n"));
            state.switch_stack.push(switch_depth);
            state.depth = switch_depth + 2;
            i += 2;
            continue;
        }
        emit_switch_stmt(current, state, output);
        i += 1;
    }
}

fn emit_php_echo(code: &str, pad: &str, state: &mut PhpDepthState, output: &mut String) {
    if is_echo_block_closer(code) {
        state.depth = state.depth.saturating_sub(1);
        let pad = INDENT.repeat(state.depth);
        output.push_str(&format_echo(code, &pad));
    } else {
        output.push_str(&format_echo(code, pad));
        if is_echo_block_opener(code) {
            state.depth += 1;
        }
    }
}

fn format_nodes(nodes: &[Node], depth: usize, output: &mut String) {
    let mut state = PhpDepthState {
        depth,
        switch_stack: Vec::new(),
    };
    let mut i = 0usize;
    while i < nodes.len() {
        let pad = INDENT.repeat(state.depth);

        match &nodes[i] {
            Node::Element {
                name,
                attributes,
                children,
            } => {
                emit_element(name, attributes, children, (state.depth, output));
            }
            Node::Text(s) => {
                let trimmed = s.trim();
                if !trimmed.is_empty() {
                    output.push_str(&format!("{pad}{trimmed}\n"));
                } else if state.depth <= 1 && s.contains('\n') && s.chars().filter(|&c| c == '\n').count() > 1 {
                    output.push('\n');
                }
            }
            Node::PhpBlock(code) => {
                if state.depth == 0 && (is_header_php_block(code) || is_docblock_only(code)) {
                    let mut merged = code.trim().to_string();
                    let mut j = i + 1;
                    let mut merged_any = false;

                    while j < nodes.len() {
                        match &nodes[j] {
                            Node::Text(s) if s.trim().is_empty() => {
                                j += 1;
                            }
                            Node::PhpBlock(next_code)
                                if is_header_php_block(next_code) || is_docblock_only(next_code) =>
                            {
                                if !merged.is_empty() {
                                    merged.push('\n');
                                }
                                merged.push_str(next_code.trim());
                                merged_any = true;
                                j += 1;
                            }
                            _ => break,
                        }
                    }

                    if merged_any {
                        emit_php_block(&merged, &pad, &mut state, output);
                        i = j;
                        continue;
                    }
                }
                emit_php_block(code, &pad, &mut state, output);
            }
            Node::PhpEcho(code) => emit_php_echo(code, &pad, &mut state, output),
            Node::Doctype(s) => {
                output.push_str(&format!("{pad}<!DOCTYPE {s}>\n"));
            }
            Node::Comment(s) => {
                output.push_str(&format!("{pad}<!-- {s} -->\n"));
            }
        }

        i += 1;
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
