use super::Formatter;
use super::docblock::{emit_docblock_php, expand_single_line_docblock, is_docblock_only};
use super::echo::{contains_break, is_echo_block_closer, is_echo_block_opener, is_single_echo_block};
use super::indent::{
    count_semicolons_outside_parens, has_switch_case, is_header_php_block, is_php_block_closer, is_php_block_opener,
    is_switch_case_peer, split_header_and_opener, visual_len,
};
use super::php::{format_php_code, join_php_lines};
use super::split::find_ternary_positions;
use crate::parser::ast::Node;
use crate::parser::lexer::Attribute;

const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source", "track", "wbr",
];

const RAW_TEXT_ELEMENTS: &[&str] = &["script", "style"];

const VERBATIM_ELEMENTS: &[&str] = &["textarea", "pre"];

fn is_void_element(name: &str) -> bool {
    VOID_ELEMENTS.contains(&name.to_lowercase().as_str())
}

fn is_verbatim_element(name: &str) -> bool {
    VERBATIM_ELEMENTS.contains(&name.to_lowercase().as_str())
}

fn push_raw_text_lines(s: &str, pad: &str, output: &mut String) {
    let trimmed = s.trim_start_matches('\n').trim_end();
    if trimmed.is_empty() {
        return;
    }
    for line in trimmed.lines() {
        if line.chars().next().is_some_and(char::is_whitespace) {
            output.push_str(line);
        } else {
            output.push_str(pad);
            output.push_str(line);
        }
        output.push('\n');
    }
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

impl Formatter {
    fn emit_open_tag(&self, name: &str, attributes: &[Attribute], pad: &str, output: &mut String) {
        let attrs = format_attributes(attributes);
        let single = format!("{pad}<{name}{attrs}>");

        if attributes.is_empty() || visual_len(&single) <= self.max_line_length {
            output.push_str(&single);
            output.push('\n');
            return;
        }

        let indent = &self.indent;
        output.push_str(&format!("{pad}<{name}\n"));
        let attr_pad = format!("{pad}{indent}");
        for attr in attributes {
            output.push_str(&attr_pad);
            output.push_str(&format_attribute(attr));
            output.push('\n');
        }
        output.push_str(&format!("{pad}>\n"));
    }
}

const BLOCK_ELEMENTS: &[&str] = &[
    "div",
    "section",
    "article",
    "main",
    "aside",
    "nav",
    "header",
    "footer",
    "form",
    "fieldset",
    "details",
    "figure",
    "figcaption",
    "dl",
    "ol",
    "ul",
    "table",
    "thead",
    "tbody",
    "tfoot",
    "tr",
];

fn is_block_element(name: &str) -> bool {
    BLOCK_ELEMENTS.contains(&name.to_lowercase().as_str())
}

fn is_inline_content(children: &[Node]) -> bool {
    children.iter().all(|c| match c {
        Node::Text(_) | Node::PhpEcho(_) => true,
        Node::PhpBlock(code) => is_single_echo_block(code),
        _ => false,
    })
}

fn collapse_whitespace(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut prev_ws = false;
    for ch in s.chars() {
        if ch.is_whitespace() {
            if !prev_ws {
                result.push(' ');
            }
            prev_ws = true;
        } else {
            result.push(ch);
            prev_ws = false;
        }
    }
    result
}

fn format_inline_content(children: &[Node]) -> String {
    let raw: String = children
        .iter()
        .map(|c| match c {
            Node::Text(s) => collapse_whitespace(s),
            Node::PhpEcho(s) => format!("<?= {} ?>", format_php_code(&join_php_lines(s))),
            Node::PhpBlock(s) if is_single_echo_block(s) => {
                let expr = s.trim().strip_prefix("echo ").unwrap_or(s);
                let expr = expr.strip_suffix(';').unwrap_or(expr).trim();
                format!("<?= {} ?>", format_php_code(expr))
            }
            _ => String::new(),
        })
        .collect();
    raw.trim().to_string()
}

fn format_inline(name: &str, attributes: &[Attribute], children: &[Node]) -> String {
    let attrs = format_attributes(attributes);
    let content = format_inline_content(children);
    format!("<{name}{attrs}>{content}</{name}>")
}

impl Formatter {
    fn emit_verbatim_element(
        &self,
        name: &str,
        attributes: &[Attribute],
        children: &[Node],
        ctx: (usize, &mut String),
    ) {
        let (depth, output) = ctx;
        let pad = self.indent.repeat(depth);
        let attrs = format_attributes(attributes);
        let content: String = children
            .iter()
            .filter_map(|c| match c {
                Node::Text(s) => Some(s.as_str()),
                _ => None,
            })
            .collect();
        output.push_str(&format!("{pad}<{name}{attrs}>{content}</{name}>\n"));
    }

    fn emit_raw_text_element(
        &self,
        name: &str,
        attributes: &[Attribute],
        children: &[Node],
        ctx: (usize, &mut String),
    ) {
        let (depth, output) = ctx;
        let pad = self.indent.repeat(depth);
        self.emit_open_tag(name, attributes, &pad, output);
        for child in children {
            if let Node::Text(s) = child {
                push_raw_text_lines(s, &pad, output);
            }
        }
        output.push_str(&format!("{pad}</{name}>\n"));
    }

    fn emit_element(&self, name: &str, attributes: &[Attribute], children: &[Node], ctx: (usize, &mut String)) {
        let (depth, output) = ctx;
        if is_verbatim_element(name) {
            self.emit_verbatim_element(name, attributes, children, (depth, output));
            return;
        }
        if RAW_TEXT_ELEMENTS.contains(&name.to_lowercase().as_str()) {
            self.emit_raw_text_element(name, attributes, children, (depth, output));
            return;
        }
        let indent = &self.indent;
        let pad = indent.repeat(depth);
        if children.is_empty() && is_void_element(name) {
            self.emit_open_tag(name, attributes, &pad, output);
        } else if children.is_empty()
            || children
                .iter()
                .all(|c| matches!(c, Node::Text(s) if s.trim().is_empty()))
        {
            let attrs = format_attributes(attributes);
            let inline_tag = format!("{pad}<{name}{attrs}></{name}>");
            if visual_len(&inline_tag) <= self.max_line_length {
                output.push_str(&inline_tag);
                output.push('\n');
            } else {
                self.emit_open_tag(name, attributes, &pad, output);
                output.push_str(&format!("{pad}</{name}>\n"));
            }
        } else if is_inline_content(children)
            && (!is_block_element(name)
                || children
                    .iter()
                    .filter(|c| matches!(c, Node::PhpEcho(_) | Node::PhpBlock(_)))
                    .count()
                    <= 1)
        {
            let inline = format_inline(name, attributes, children);
            if visual_len(&pad) + visual_len(&inline) <= self.max_line_length {
                output.push_str(&pad);
                output.push_str(&inline);
                output.push('\n');
            } else {
                let content = format_inline_content(children);
                let inner_pad = format!("{pad}{indent}");
                let content_line = format!("{inner_pad}{content}");
                let has_text = children
                    .iter()
                    .any(|c| matches!(c, Node::Text(s) if !s.trim().is_empty()));
                if visual_len(&content_line) <= self.max_line_length || (!is_block_element(name) && has_text) {
                    self.emit_open_tag(name, attributes, &pad, output);
                    output.push_str(&content_line);
                    output.push('\n');
                    output.push_str(&format!("{pad}</{name}>\n"));
                } else {
                    self.emit_open_tag(name, attributes, &pad, output);
                    self.format_nodes(children, depth + 1, output);
                    output.push_str(&format!("{pad}</{name}>\n"));
                }
            }
        } else {
            self.emit_open_tag(name, attributes, &pad, output);
            self.format_nodes(children, depth + 1, output);
            output.push_str(&format!("{pad}</{name}>\n"));
        }
    }
}

#[derive(Clone)]
struct PhpDepthState {
    depth: usize,
    switch_stack: Vec<usize>,
}

impl Formatter {
    fn emit_switch_stmt(&self, trimmed: &str, state: &mut PhpDepthState, output: &mut String) {
        let indent = &self.indent;
        let formatted = format_php_code(trimmed);
        let lower = trimmed.to_lowercase();
        if lower.starts_with("switch") && is_php_block_opener(trimmed) {
            let pad = indent.repeat(state.depth);
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
            let pad = indent.repeat(case_lvl);
            output.push_str(&format!("{pad}<?php {formatted} ?>\n"));
            state.depth = case_lvl + 1;
        } else if lower.starts_with("endswitch") {
            let lvl = state.switch_stack.pop().unwrap_or(state.depth.saturating_sub(1));
            let pad = indent.repeat(lvl);
            output.push_str(&format!("{pad}<?php {formatted} ?>\n"));
            state.depth = lvl;
        } else if contains_break(&lower) {
            let pad = indent.repeat(state.depth);
            output.push_str(&format!("{pad}<?php {formatted} ?>\n"));
        } else {
            let pad = indent.repeat(state.depth);
            output.push_str(&format!("{pad}<?php {formatted} ?>\n"));
            if is_php_block_opener(trimmed) {
                state.depth += 1;
            }
        }
    }

    fn emit_multiline_php(&self, code: &str, pad: &str, depth: &mut usize, output: &mut String) {
        let is_header = is_header_php_block(code);
        if is_header {
            if let Some((header_code, opener_line)) = split_header_and_opener(code) {
                output.push_str(&format!("{pad}<?php\n"));
                let reindented = self.reindent_php_block(&header_code, pad);
                output.push_str(&reindented);
                output.push('\n');
                output.push_str(&format!("{pad}?>\n"));
                let formatted = format_php_code(&opener_line);
                output.push_str(&format!("{pad}<?php {formatted} ?>\n"));
                *depth += 1;
                return;
            }
            output.push_str(&format!("{pad}<?php\n"));
            let reindented = self.reindent_php_block(code, pad);
            output.push_str(&reindented);
            output.push('\n');
            output.push_str(&format!("{pad}?>\n"));
        } else {
            self.emit_multiline_php_inline(code, pad, output);
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

    fn emit_multiline_php_inline(&self, code: &str, pad: &str, output: &mut String) {
        let reindented = self.reindent_php_block(code, pad);
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

    fn emit_single_php(&self, code: &str, pad: &str, state: &mut PhpDepthState, output: &mut String) {
        let indent = &self.indent;
        let formatted = format_php_code(code);
        let lower = code.trim().to_lowercase();
        if lower.starts_with("switch") && is_php_block_opener(code) && !is_php_block_closer(code) {
            let stmt_pad = indent.repeat(state.depth);
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
            let stmt_pad = indent.repeat(case_lvl);
            output.push_str(&format!("{stmt_pad}<?php {formatted} ?>\n"));
            state.depth = case_lvl + 1;
        } else if !state.switch_stack.is_empty() && lower.starts_with("endswitch") {
            let lvl = state.switch_stack.pop().unwrap_or(state.depth.saturating_sub(1));
            let stmt_pad = indent.repeat(lvl);
            output.push_str(&format!("{stmt_pad}<?php {formatted} ?>\n"));
            state.depth = lvl;
        } else if !state.switch_stack.is_empty() && contains_break(&lower) {
            let stmt_pad = indent.repeat(state.depth);
            output.push_str(&format!("{stmt_pad}<?php {formatted} ?>\n"));
        } else if is_php_block_closer(code) {
            state.depth = state.depth.saturating_sub(1);
            let pad_less = indent.repeat(state.depth);
            output.push_str(&format!("{pad_less}<?php {formatted} ?>\n"));
            if is_php_block_opener(code) {
                state.depth += 1;
            }
        } else {
            self.emit_single_php_long(code, pad, &mut state.depth, output);
        }
    }

    fn emit_single_php_long(&self, code: &str, pad: &str, depth: &mut usize, output: &mut String) {
        let indent = &self.indent;
        let formatted = format_php_code(code);
        if is_header_php_block(code) {
            output.push_str(&format!("{pad}<?php\n"));
            let reindented = self.reindent_php_block(code, pad);
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
        let is_alt_syntax_opener = code.trim().ends_with(':');
        if visual_len(&single) <= self.max_line_length || is_alt_syntax_opener {
            output.push_str(&format!("{single}\n"));
            if is_php_block_opener(code) {
                *depth += 1;
            }
            return;
        }
        if let Some((q_pos, c_pos)) = find_ternary_positions(&formatted) {
            let condition = formatted[..q_pos].trim_end();
            let true_val = formatted[q_pos + 1..c_pos].trim();
            let false_val = formatted[c_pos + 1..].trim();
            let inner_pad = format!("{pad}{indent}");
            output.push_str(&format!(
                "{pad}<?php {condition}\n{inner_pad}? {true_val}\n{inner_pad}: {false_val} ?>\n"
            ));
        } else {
            let reindented = self.reindent_php_block(code, pad);
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

    fn emit_php_block(&self, code: &str, pad: &str, state: &mut PhpDepthState, output: &mut String) {
        let trimmed = code.trim();
        if let Some(expr) = trimmed.strip_prefix("echo ") {
            let expr = expr.strip_suffix(';').unwrap_or(expr).trim();
            let semicolons = count_semicolons_outside_parens(code);
            if semicolons <= 1 && !expr.contains('\n') {
                self.emit_php_echo(expr, pad, state, output);
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
            self.emit_php_switch_block(code, state, output);
        } else if is_multiline {
            self.emit_multiline_php(code, pad, &mut state.depth, output);
        } else {
            self.emit_single_php(code, pad, state, output);
        }
    }

    fn emit_php_switch_block(&self, code: &str, state: &mut PhpDepthState, output: &mut String) {
        let indent = &self.indent;
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
                let stmt_pad = indent.repeat(switch_depth);
                let switch_stmt = format_php_code(current);
                let first_case = format_php_code(statements[i + 1]);
                let case_pad = format!("{stmt_pad}{indent}");
                output.push_str(&format!("{stmt_pad}<?php {switch_stmt}\n{case_pad}{first_case} ?>\n"));
                state.switch_stack.push(switch_depth);
                state.depth = switch_depth + 2;
                i += 2;
                continue;
            }
            self.emit_switch_stmt(current, state, output);
            i += 1;
        }
    }

    fn emit_php_echo(&self, code: &str, pad: &str, state: &mut PhpDepthState, output: &mut String) {
        if is_echo_block_closer(code) {
            state.depth = state.depth.saturating_sub(1);
            let pad = self.indent.repeat(state.depth);
            output.push_str(&self.format_echo(code, &pad));
        } else {
            output.push_str(&self.format_echo(code, pad));
            if is_echo_block_opener(code) {
                state.depth += 1;
            }
        }
    }
}

const INLINE_ELEMENTS: &[&str] = &[
    "a", "abbr", "b", "bdi", "bdo", "br", "cite", "code", "data", "del", "dfn", "em", "i", "ins", "kbd", "mark", "q",
    "s", "samp", "small", "span", "strong", "sub", "sup", "time", "u", "var", "wbr",
];

fn is_truly_inline_element(name: &str) -> bool {
    INLINE_ELEMENTS.contains(&name.to_lowercase().as_str())
}

fn is_inline_element_node(node: &Node) -> bool {
    match node {
        Node::Element { name, children, .. } => {
            is_truly_inline_element(name) && (is_void_element(name) || is_inline_content(children))
        }
        _ => false,
    }
}

fn render_node_inline(node: &Node) -> String {
    match node {
        Node::Text(s) => collapse_whitespace(s),
        Node::PhpEcho(s) => format!("<?= {} ?>", format_php_code(&join_php_lines(s))),
        Node::PhpBlock(s) if is_single_echo_block(s) => {
            let expr = s.trim().strip_prefix("echo ").unwrap_or(s);
            let expr = expr.strip_suffix(';').unwrap_or(expr).trim();
            format!("<?= {} ?>", format_php_code(expr))
        }
        Node::Element {
            name,
            attributes,
            children,
        } => {
            if is_void_element(name) {
                let attrs = format_attributes(attributes);
                format!("<{name}{attrs}>")
            } else {
                format_inline(name, attributes, children)
            }
        }
        _ => String::new(),
    }
}

fn collect_inline_run(nodes: &[Node], start: usize) -> Option<usize> {
    let mut end = start;
    let mut has_text = false;
    let mut has_echo = false;
    let mut has_inline_elem = false;
    while end < nodes.len() {
        match &nodes[end] {
            Node::Text(s) => {
                if !s.trim().is_empty() {
                    has_text = true;
                } else if s.contains('\n') && (has_echo || has_inline_elem) {
                    end += 1;
                    break;
                }
                end += 1;
            }
            Node::PhpEcho(code) => {
                if is_echo_block_opener(code) || is_echo_block_closer(code) {
                    break;
                }
                has_echo = true;
                end += 1;
            }
            Node::Element { name, .. } if is_inline_element_node(&nodes[end]) => {
                has_inline_elem = true;
                end += 1;
                if is_void_element(name) && name.eq_ignore_ascii_case("br") {
                    break;
                }
            }
            _ => break,
        }
    }
    if has_echo && (has_text || has_inline_elem) && end > start + 1 {
        Some(end)
    } else {
        None
    }
}

impl Formatter {
    fn emit_inline_run(&self, nodes: &[Node], pad: &str, depth: usize, output: &mut String) {
        let raw: String = nodes.iter().map(render_node_inline).collect();
        let content = raw.trim().to_string();
        if content.is_empty() {
            return;
        }
        let line = format!("{pad}{content}");
        if visual_len(&line) <= self.max_line_length {
            output.push_str(&line);
            output.push('\n');
        } else {
            for node in nodes {
                self.format_nodes(std::slice::from_ref(node), depth, output);
            }
        }
    }
}

fn try_merge_header_blocks(nodes: &[Node], start: usize, code: &str) -> Option<(String, usize)> {
    if !is_header_php_block(code) && !is_docblock_only(code) {
        return None;
    }
    let mut merged = code.trim().to_string();
    let mut j = start + 1;
    let mut merged_any = false;
    while j < nodes.len() {
        match &nodes[j] {
            Node::Text(s) if s.trim().is_empty() => j += 1,
            Node::PhpBlock(next) if is_header_php_block(next) || is_docblock_only(next) => {
                if !merged.is_empty() {
                    merged.push('\n');
                }
                merged.push_str(next.trim());
                merged_any = true;
                j += 1;
            }
            _ => break,
        }
    }
    merged_any.then_some((merged, j))
}

impl Formatter {
    fn format_nodes(&self, nodes: &[Node], depth: usize, output: &mut String) {
        let mut state = PhpDepthState {
            depth,
            switch_stack: Vec::new(),
        };
        let mut i = 0usize;
        while i < nodes.len() {
            let pad = self.indent.repeat(state.depth);

            let mut node_output = String::new();
            let mut node_state = state.clone();
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.emit_one(nodes, i, &pad, (&mut node_state, &mut node_output))
            }));

            match result {
                Ok(next) => {
                    output.push_str(&node_output);
                    state = node_state;
                    i = next;
                }
                Err(_) => {
                    eprintln!("phew: предупреждение: блок не удалось отформатировать, оставлен как есть");
                    self.render_node_raw(&nodes[i], &pad, output);
                    i += 1;
                }
            }
        }
    }

    fn emit_one(&self, nodes: &[Node], i: usize, pad: &str, ctx: (&mut PhpDepthState, &mut String)) -> usize {
        let (state, output) = ctx;
        if matches!(&nodes[i], Node::Text(_) | Node::PhpEcho(_)) || is_inline_element_node(&nodes[i]) {
            if let Some(end) = collect_inline_run(nodes, i) {
                self.emit_inline_run(&nodes[i..end], pad, state.depth, output);
                return end;
            }
        }

        match &nodes[i] {
            Node::Element {
                name,
                attributes,
                children,
            } => {
                self.emit_element(name, attributes, children, (state.depth, output));
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
                if state.depth == 0 {
                    if let Some((merged, j)) = try_merge_header_blocks(nodes, i, code) {
                        self.emit_php_block(&merged, pad, state, output);
                        return j;
                    }
                }
                self.emit_php_block(code, pad, state, output);
            }
            Node::PhpEcho(code) => self.emit_php_echo(code, pad, state, output),
            Node::Doctype(s) => output.push_str(&format!("{pad}<!DOCTYPE {s}>\n")),
            Node::Comment(s) => output.push_str(&format!("{pad}<!-- {s} -->\n")),
        }

        i + 1
    }

    fn render_node_raw(&self, node: &Node, pad: &str, output: &mut String) {
        match node {
            Node::Text(s) => {
                let trimmed = s.trim();
                if !trimmed.is_empty() {
                    output.push_str(&format!("{pad}{trimmed}\n"));
                }
            }
            Node::PhpBlock(code) => output.push_str(&format!("{pad}<?php {} ?>\n", code.trim())),
            Node::PhpEcho(code) => output.push_str(&format!("{pad}<?= {} ?>\n", code.trim())),
            Node::Doctype(s) => output.push_str(&format!("{pad}<!DOCTYPE {s}>\n")),
            Node::Comment(s) => output.push_str(&format!("{pad}<!-- {s} -->\n")),
            Node::Element {
                name,
                attributes,
                children,
            } => {
                let attrs = format_attributes(attributes);
                output.push_str(&format!("{pad}<{name}{attrs}>\n"));
                if children.is_empty() && is_void_element(name) {
                    return;
                }
                let child_pad = format!("{pad}{}", self.indent);
                for child in children {
                    self.render_node_raw(child, &child_pad, output);
                }
                output.push_str(&format!("{pad}</{name}>\n"));
            }
        }
    }

    pub fn format(&self, nodes: &[Node]) -> String {
        let mut output = String::new();
        self.format_nodes(nodes, 0, &mut output);
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{ast, lexer};
    use pretty_assertions::assert_eq;

    fn format_str(input: &str) -> String {
        let tokens = lexer::tokenize(input);
        let nodes = ast::parse(tokens);
        Formatter::default().format(&nodes)
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
        assert_eq!(format_str("<br />"), "<br>\n");
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

    #[test]
    fn cyrillic_assignment_array_splits() {
        let input = "<?php $абв = ['первыйКлюч' => 'значение один', 'второйКлюч' => 'значение два', 'третийКлюч' => 'значение три', 'четвёртыйКлюч' => 'значение четыре']; ?>";
        let expected = "\
<?php $абв = [
    'первыйКлюч' => 'значение один',
    'второйКлюч' => 'значение два',
    'третийКлюч' => 'значение три',
    'четвёртыйКлюч' => 'значение четыре',
]; ?>
";
        assert_eq!(format_str(input), expected);
    }

    #[test]
    fn cyrillic_nested_array_fat_arrow() {
        let input = "<?php $н = ['заголовок' => 'Главная страница каталога', 'параметры' => ['ширина' => 'сто двадцать', 'высота' => 'восемьдесят пять', 'отступ' => 'десять']]; ?>";
        let expected = "\
<?php $н = [
    'заголовок' => 'Главная страница каталога',
    'параметры' => [
        'ширина' => 'сто двадцать',
        'высота' => 'восемьдесят пять',
        'отступ' => 'десять',
    ],
]; ?>
";
        assert_eq!(format_str(input), expected);
    }

    #[test]
    fn empty_docblock_does_not_panic() {
        let input = "<?php /**/ $оченьДлинноеИмяПеременной = 'очень длинное строковое значение которое превышает лимит ширины в сто двадцать'; ?>";
        let out = format_str(input);
        assert!(out.contains("/**/"));
        assert!(out.contains("$оченьДлинноеИмяПеременной"));
    }

    #[test]
    fn render_node_raw_preserves_php_block() {
        let mut out = String::new();
        Formatter::default().render_node_raw(&Node::PhpBlock("$x = 1;".into()), "  ", &mut out);
        assert_eq!(out, "  <?php $x = 1; ?>\n");
    }

    #[test]
    fn render_node_raw_preserves_element_subtree() {
        let node = Node::Element {
            name: "div".into(),
            attributes: vec![Attribute {
                name: "class".into(),
                value: Some("каталог".into()),
            }],
            children: vec![Node::PhpEcho("$товар".into())],
        };
        let mut out = String::new();
        Formatter::default().render_node_raw(&node, "", &mut out);
        let expected = "\
<div class=\"каталог\">
    <?= $товар ?>
</div>
";
        assert_eq!(out, expected);
    }

    #[test]
    fn format_never_panics_on_adversarial_input() {
        let inputs = [
            "<?php /**/ $оооооооооооооооооооооооооооооооооочень = 'длинное значение для переноса строки за лимит ширины в сто двадцать символов'; ?>",
            "<?php $ы = ['ключ' => 'значение', 'другойКлюч' => 'другое значение', 'третийКлюч' => 'третье', 'четвёртый' => 'четвёртое значение']; ?>",
            "<div><?= $переменная ?></div>",
        ];
        for input in inputs {
            let out = format_str(input);
            assert!(!out.is_empty(), "пустой вывод для: {input}");
        }
    }

    #[test]
    fn textarea_content_preserved_verbatim() {
        let input = "<div> <textarea name=\"body\"><b>x</textarea> </div>";
        let expected = "\
<div>
    <textarea name=\"body\"><b>x</textarea>
</div>
";
        assert_eq!(format_str(input), expected);
    }

    #[test]
    fn pre_whitespace_preserved_verbatim() {
        let input = "<pre>\n  a\n    b\n</pre>";
        let expected = "<pre>\n  a\n    b\n</pre>\n";
        assert_eq!(format_str(input), expected);
    }
}
