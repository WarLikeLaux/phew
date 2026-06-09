use super::Formatter;
use super::docblock::{emit_docblock_php, expand_single_line_docblock, is_docblock_only};
use super::echo::{contains_break, is_echo_block_closer, is_echo_block_opener, is_single_echo_block};
use super::indent::{
    count_semicolons_outside_parens, count_top_level_semicolons, has_switch_case, is_header_php_block,
    is_php_block_closer, is_php_block_opener, is_switch_case_peer, split_header_and_opener, visual_len,
};
use super::php::{format_php_code, join_php_lines};
use super::split::{find_ternary_positions, has_expandable_closure};
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

    let parts: Vec<String> = attrs.iter().map(format_attribute).collect();
    format!(" {}", parts.join(" "))
}

fn has_literal_quote(value: &str, quote: char) -> bool {
    let chars: Vec<char> = value.chars().collect();
    let mut in_php = false;
    let mut i = 0;
    while i < chars.len() {
        if !in_php && chars[i] == '<' && chars.get(i + 1) == Some(&'?') {
            in_php = true;
            i += 2;
        } else if in_php && chars[i] == '?' && chars.get(i + 1) == Some(&'>') {
            in_php = false;
            i += 2;
        } else if !in_php && chars[i] == quote {
            return true;
        } else {
            i += 1;
        }
    }
    false
}

fn attr_quote(value: &str) -> char {
    if has_literal_quote(value, '"') && !has_literal_quote(value, '\'') {
        '\''
    } else {
        '"'
    }
}

fn normalize_php_segment(seg: &str) -> String {
    let Some(inner) = seg.strip_suffix("?>") else {
        return seg.to_string();
    };
    if let Some(rest) = inner.strip_prefix("<?=") {
        format!("<?= {} ?>", format_php_code(&join_php_lines(rest.trim())))
    } else if let Some(rest) = inner.strip_prefix("<?php") {
        format!("<?php {} ?>", format_php_code(&join_php_lines(rest.trim())))
    } else {
        seg.to_string()
    }
}

fn find_php_close_tag(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut i = 0;
    let mut in_single = false;
    let mut in_double = false;
    while i + 1 < bytes.len() {
        let b = bytes[i];
        if (in_single || in_double) && b == b'\\' {
            i += 2;
            continue;
        }
        match b {
            b'\'' if !in_double => in_single = !in_single,
            b'"' if !in_single => in_double = !in_double,
            b'?' if !in_single && !in_double && bytes[i + 1] == b'>' => return Some(i),
            _ => {}
        }
        i += 1;
    }
    None
}

fn normalize_attr_value(value: &str) -> String {
    if !value.contains("<?") {
        return value.to_string();
    }
    let mut out = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(start) = rest.find("<?") {
        out.push_str(&rest[..start]);
        let after = &rest[start..];
        let Some(close) = find_php_close_tag(after) else {
            out.push_str(after);
            return out;
        };
        out.push_str(&normalize_php_segment(&after[..close + 2]));
        rest = &after[close + 2..];
    }
    out.push_str(rest);
    out
}

fn single_php_segment(value: &str) -> Option<(String, bool, String, String)> {
    let start = value.find("<?")?;
    let after = &value[start..];
    let is_echo = after.starts_with("<?=");
    let is_php = after.starts_with("<?php");
    if !is_echo && !is_php {
        return None;
    }
    let close = find_php_close_tag(after)?;
    let suffix = &after[close + 2..];
    if suffix.contains("<?") {
        return None;
    }
    let inner = &after[..close];
    let code = if is_echo {
        inner.strip_prefix("<?=")?.trim()
    } else {
        inner.strip_prefix("<?php")?.trim()
    };
    Some((
        value[..start].to_string(),
        is_echo,
        code.to_string(),
        suffix.to_string(),
    ))
}

fn format_attribute(attr: &Attribute) -> String {
    let Some(value) = &attr.value else {
        return attr.name.clone();
    };
    let value = normalize_attr_value(value);
    let quote = attr_quote(&value);
    format!("{}={quote}{value}{quote}", attr.name)
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
            let line = format!("{attr_pad}{}", format_attribute(attr));
            if visual_len(&line) <= self.max_line_length || !self.emit_attribute_split(attr, &attr_pad, output) {
                output.push_str(&line);
                output.push('\n');
            }
        }
        output.push_str(&format!("{pad}>\n"));
    }

    fn emit_attribute_split(&self, attr: &Attribute, attr_pad: &str, output: &mut String) -> bool {
        let Some(raw) = &attr.value else {
            return false;
        };
        let value = normalize_attr_value(raw);
        let Some((prefix, is_echo, code, suffix)) = single_php_segment(&value) else {
            return false;
        };
        let Some(split) = self.try_split_long_line(&code, attr_pad) else {
            return false;
        };
        let lines: Vec<&str> = split.lines().filter(|l| !l.trim().is_empty()).collect();
        if lines.len() < 2 {
            return false;
        }
        let quote = attr_quote(&value);
        let open = if is_echo { "<?=" } else { "<?php" };
        output.push_str(&format!(
            "{attr_pad}{}={quote}{prefix}{open} {}\n",
            attr.name,
            lines[0].trim_start()
        ));
        for line in &lines[1..lines.len() - 1] {
            output.push_str(line);
            output.push('\n');
        }
        output.push_str(&format!("{} ?>{suffix}{quote}\n", lines[lines.len() - 1]));
        true
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
        Node::Element { .. } | Node::Doctype(_) | Node::Comment(_) => false,
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
            Node::PhpBlock(_) | Node::Element { .. } | Node::Doctype(_) | Node::Comment(_) => String::new(),
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

fn is_self_contained_brace_switch(code: &str) -> bool {
    let trimmed = code.trim();
    if !trimmed.to_lowercase().starts_with("switch") || !trimmed.ends_with('}') {
        return false;
    }
    let chars: Vec<char> = trimmed.chars().collect();
    let mut braces = 0i32;
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '\'' || ch == '"' {
            i += 1;
            while i < chars.len() && chars[i] != ch {
                if chars[i] == '\\' {
                    i += 1;
                }
                i += 1;
            }
        } else if ch == '{' {
            braces += 1;
        } else if ch == '}' {
            braces -= 1;
        }
        i += 1;
    }
    braces == 0
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
        } else if trimmed == "}" && !state.switch_stack.is_empty() {
            let lvl = state.switch_stack.pop().unwrap_or(state.depth.saturating_sub(1));
            let pad = indent.repeat(lvl + 1);
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
            if lines[0].trim_start().starts_with("/**") {
                output.push_str(&format!("{pad}<?php\n"));
                for line in &lines[..lines.len() - 1] {
                    output.push_str(line);
                    output.push('\n');
                }
            } else {
                output.push_str(&format!("{pad}<?php {}\n", lines[0].trim_start()));
                for line in &lines[1..lines.len() - 1] {
                    output.push_str(line);
                    output.push('\n');
                }
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
        } else if !state.switch_stack.is_empty() && code.trim() == "}" {
            let lvl = state.switch_stack.pop().unwrap_or(state.depth.saturating_sub(1));
            let stmt_pad = indent.repeat(lvl);
            output.push_str(&format!("{stmt_pad}<?php {formatted} ?>\n"));
            state.depth = lvl;
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
        if (visual_len(&single) <= self.max_line_length && !has_expandable_closure(&formatted)) || is_alt_syntax_opener
        {
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
        } else if let Some(split) = self
            .try_split_long_line(&formatted, pad)
            .or_else(|| self.expand_braced_value(&formatted, pad))
        {
            let lines: Vec<&str> = split.lines().filter(|l| !l.trim().is_empty()).collect();
            if lines.len() > 1 {
                output.push_str(&format!("{pad}<?php {}\n", lines[0].trim_start()));
                for line in &lines[1..lines.len() - 1] {
                    output.push_str(line);
                    output.push('\n');
                }
                output.push_str(&format!("{} ?>\n", lines[lines.len() - 1]));
            } else if let Some(one) = lines.first() {
                output.push_str(&format!("{pad}<?php {} ?>\n", one.trim_start()));
            }
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
        let semicolons = count_top_level_semicolons(code);
        let is_multiline = code.contains('\n') || semicolons > 1 || has_switch_case(code);
        if is_multiline && has_switch_case(code) && !is_self_contained_brace_switch(code) {
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

fn is_inline_run_start(node: &Node) -> bool {
    matches!(node, Node::Text(_) | Node::PhpEcho(_))
        || matches!(node, Node::PhpBlock(code) if is_single_echo_block(code))
        || is_inline_element_node(node)
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
        Node::PhpBlock(_) | Node::Doctype(_) | Node::Comment(_) => String::new(),
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
            Node::PhpBlock(code) if is_single_echo_block(code) => {
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

fn render_inline_run(nodes: &[Node]) -> String {
    nodes
        .iter()
        .map(render_node_inline)
        .collect::<String>()
        .trim()
        .to_string()
}

fn is_echo_like(node: &Node) -> bool {
    matches!(node, Node::PhpEcho(_)) || matches!(node, Node::PhpBlock(code) if is_single_echo_block(code))
}

fn is_inline_atom(node: &Node) -> bool {
    is_echo_like(node) || is_inline_element_node(node)
}

fn line_reparses_as_run(nodes: &[Node]) -> bool {
    if nodes.len() == 1 {
        return true;
    }
    let has_echo = nodes.iter().any(is_echo_like);
    let has_anchor = nodes
        .iter()
        .any(|node| matches!(node, Node::Text(s) if !s.trim().is_empty()) || is_inline_element_node(node));
    has_echo && has_anchor
}

fn inline_segments(nodes: &[Node]) -> Vec<usize> {
    let mut bounds = Vec::new();
    for (idx, node) in nodes.iter().enumerate() {
        if is_inline_atom(node) {
            bounds.push(idx + 1);
        }
    }
    if bounds.last() != Some(&nodes.len()) {
        bounds.push(nodes.len());
    }
    bounds
}

impl Formatter {
    fn inline_run_fits(&self, nodes: &[Node], pad: &str) -> bool {
        visual_len(pad) + visual_len(&render_inline_run(nodes)) <= self.max_line_length
    }

    fn emit_inline_run(&self, nodes: &[Node], pad: &str, depth: usize, output: &mut String) {
        let content = render_inline_run(nodes);
        if content.is_empty() {
            return;
        }
        if visual_len(pad) + visual_len(&content) <= self.max_line_length {
            output.push_str(pad);
            output.push_str(&content);
            output.push('\n');
            return;
        }

        let bounds = inline_segments(nodes);
        let mut line_start = 0;
        let mut line_end = bounds[0];
        for &seg_end in &bounds[1..] {
            let candidate = &nodes[line_start..seg_end];
            if self.inline_run_fits(candidate, pad) && line_reparses_as_run(candidate) {
                line_end = seg_end;
            } else {
                self.emit_inline_line(&nodes[line_start..line_end], pad, depth, output);
                line_start = line_end;
                line_end = seg_end;
            }
        }
        self.emit_inline_line(&nodes[line_start..line_end], pad, depth, output);
    }

    fn emit_inline_line(&self, nodes: &[Node], pad: &str, depth: usize, output: &mut String) {
        if nodes.len() > 1 && self.inline_run_fits(nodes, pad) && line_reparses_as_run(nodes) {
            output.push_str(pad);
            output.push_str(&render_inline_run(nodes));
            output.push('\n');
            return;
        }
        for node in nodes {
            self.format_nodes(std::slice::from_ref(node), depth, output);
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
        if is_inline_run_start(&nodes[i]) {
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
                } else if i > 0 && state.depth <= 1 && s.contains('\n') && s.chars().filter(|&c| c == '\n').count() > 1
                {
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
#[path = "engine_tests.rs"]
mod tests;
