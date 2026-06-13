use super::Formatter;
use super::attrs::format_attributes;
use super::docblock::is_docblock_only;
use super::echo::{contains_line_comment, is_echo_block_closer, is_echo_block_opener, is_single_echo_block};
use super::indent::{is_header_php_block, is_php_block_opener, is_switch_case_peer, visual_len};
use super::php::{format_php_code, join_php_lines};
use super::php_emit::PhpDepthState;
use super::scan::contains_heredoc;
use crate::parser::ast::Node;
use crate::parser::lexer::Attribute;

const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source", "track", "wbr",
];

const SVG_VOID_ELEMENTS: &[&str] = &[
    "circle", "ellipse", "line", "path", "polygon", "polyline", "rect", "stop", "use",
];

const RAW_TEXT_ELEMENTS: &[&str] = &["script", "style"];

const VERBATIM_ELEMENTS: &[&str] = &["textarea", "pre"];

fn is_void_element(name: &str) -> bool {
    VOID_ELEMENTS.contains(&name.to_lowercase().as_str())
}

fn is_svg_void_element(name: &str) -> bool {
    SVG_VOID_ELEMENTS.contains(&name.to_lowercase().as_str())
}

fn is_verbatim_element(name: &str) -> bool {
    VERBATIM_ELEMENTS.contains(&name.to_lowercase().as_str())
}

fn render_comment(body: &str) -> String {
    if body.starts_with('[') || body.starts_with("<!") {
        format!("<!--{body}-->")
    } else {
        format!("<!-- {body} -->")
    }
}

fn leading_whitespace_len(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

fn push_raw_text_lines(s: &str, pad: &str, output: &mut String) {
    let lines: Vec<&str> = s.lines().collect();
    let Some(start) = lines.iter().position(|line| !line.trim().is_empty()) else {
        return;
    };
    let end = lines.iter().rposition(|line| !line.trim().is_empty()).unwrap_or(start);
    let body = &lines[start..=end];
    let min_indent = body
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| leading_whitespace_len(line))
        .min()
        .unwrap_or(0);
    for line in body {
        if line.trim().is_empty() {
            output.push('\n');
            continue;
        }
        output.push_str(pad);
        output.push_str(line.get(min_indent..).unwrap_or("").trim_end());
        output.push('\n');
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
        Node::Text(_) => true,
        Node::PhpEcho(code) => is_inline_echo_code(code),
        Node::PhpBlock(code) => is_single_echo_block(code),
        Node::Element { .. } | Node::Doctype(_) | Node::Comment(_) => false,
    })
}

fn is_inline_echo_code(code: &str) -> bool {
    !code.contains('\n') && !contains_heredoc(code) && !contains_line_comment(code)
}

fn is_inline_flow(children: &[Node]) -> bool {
    children.iter().any(is_inline_element_node)
        && children.iter().all(|c| match c {
            Node::Text(_) => true,
            Node::Element { .. } => is_inline_element_node(c),
            Node::PhpEcho(_) | Node::PhpBlock(_) | Node::Doctype(_) | Node::Comment(_) => false,
        })
}

fn push_inline_token(groups: &mut Vec<Vec<usize>>, idx: usize, space_before: bool) {
    match groups.last_mut() {
        Some(last) if !space_before => last.push(idx),
        _ => groups.push(vec![idx]),
    }
}

fn inline_glue_groups(children: &[Node]) -> Vec<Vec<usize>> {
    let mut groups: Vec<Vec<usize>> = Vec::new();
    let mut pending_space = false;
    for (idx, node) in children.iter().enumerate() {
        if let Node::Text(s) = node {
            if s.trim().is_empty() {
                if !s.is_empty() {
                    pending_space = true;
                }
                continue;
            }
            let space_before = pending_space || s.starts_with(char::is_whitespace);
            push_inline_token(&mut groups, idx, space_before);
            pending_space = s.ends_with(char::is_whitespace);
        } else {
            push_inline_token(&mut groups, idx, pending_space);
            pending_space = false;
        }
    }
    groups
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
                Node::Element { .. } | Node::PhpBlock(_) | Node::PhpEcho(_) | Node::Doctype(_) | Node::Comment(_) => {
                    None
                }
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
        let content_pad = self.indent.repeat(depth + 1);
        self.emit_open_tag(name, attributes, &pad, output);
        for child in children {
            if let Node::Text(s) = child {
                push_raw_text_lines(s, &content_pad, output);
            }
        }
        output.push_str(&format!("{pad}</{name}>\n"));
    }

    fn emit_element(&self, el: &Node, ctx: (usize, &mut String)) {
        let Node::Element {
            name,
            attributes,
            children,
            foreign,
        } = el
        else {
            return;
        };
        let (depth, output) = ctx;
        if is_verbatim_element(name) {
            self.emit_verbatim_element(name, attributes, children, (depth, output));
            return;
        }
        if RAW_TEXT_ELEMENTS.contains(&name.to_lowercase().as_str()) {
            self.emit_raw_text_element(name, attributes, children, (depth, output));
            return;
        }
        let pad = self.indent.repeat(depth);
        let is_empty = children.is_empty()
            || children
                .iter()
                .all(|c| matches!(c, Node::Text(s) if s.trim().is_empty()));
        if children.is_empty() && is_void_element(name) {
            self.emit_open_tag(name, attributes, &pad, output);
        } else if is_empty && *foreign && is_svg_void_element(name) {
            self.emit_self_closing_tag(name, attributes, &pad, output);
        } else if is_empty {
            self.emit_empty_element(name, attributes, &pad, output);
        } else if is_inline_content(children) && fits_inline_element(name, children) {
            self.emit_inline_element(name, attributes, children, (depth, output));
        } else if is_inline_flow(children) {
            self.emit_open_tag(name, attributes, &pad, output);
            self.emit_inline_flow_children(children, depth + 1, output);
            output.push_str(&format!("{pad}</{name}>\n"));
        } else {
            self.emit_open_tag(name, attributes, &pad, output);
            self.format_nodes(children, depth + 1, output);
            output.push_str(&format!("{pad}</{name}>\n"));
        }
    }

    fn emit_inline_flow_children(&self, children: &[Node], depth: usize, output: &mut String) {
        let pad = self.indent.repeat(depth);
        for group in inline_glue_groups(children) {
            if let [single] = group.as_slice() {
                self.format_nodes(std::slice::from_ref(&children[*single]), depth, output);
            } else {
                let line: String = group.iter().map(|&i| render_node_inline(&children[i])).collect();
                let line = line.trim();
                if !line.is_empty() {
                    output.push_str(&format!("{pad}{line}\n"));
                }
            }
        }
    }

    fn emit_empty_element(&self, name: &str, attributes: &[Attribute], pad: &str, output: &mut String) {
        let attrs = format_attributes(attributes);
        let inline_tag = format!("{pad}<{name}{attrs}></{name}>");
        if visual_len(&inline_tag) <= self.max_line_length {
            output.push_str(&inline_tag);
            output.push('\n');
        } else {
            self.emit_open_tag(name, attributes, pad, output);
            output.push_str(&format!("{pad}</{name}>\n"));
        }
    }

    fn emit_inline_element(&self, name: &str, attributes: &[Attribute], children: &[Node], ctx: (usize, &mut String)) {
        let (depth, output) = ctx;
        let pad = self.indent.repeat(depth);
        let inline = format_inline(name, attributes, children);
        if visual_len(&pad) + visual_len(&inline) <= self.max_line_length {
            output.push_str(&pad);
            output.push_str(&inline);
            output.push('\n');
            return;
        }
        let content = format_inline_content(children);
        let inner_pad = format!("{pad}{}", self.indent);
        let content_line = format!("{inner_pad}{content}");
        let has_text = children
            .iter()
            .any(|c| matches!(c, Node::Text(s) if !s.trim().is_empty()));
        self.emit_open_tag(name, attributes, &pad, output);
        if visual_len(&content_line) <= self.max_line_length || (!is_block_element(name) && has_text) {
            output.push_str(&content_line);
            output.push('\n');
        } else {
            self.format_nodes(children, depth + 1, output);
        }
        output.push_str(&format!("{pad}</{name}>\n"));
    }
}

fn fits_inline_element(name: &str, children: &[Node]) -> bool {
    !is_block_element(name)
        || children
            .iter()
            .filter(|c| matches!(c, Node::PhpEcho(_) | Node::PhpBlock(_)))
            .count()
            <= 1
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
        Node::Text(_) | Node::PhpBlock(_) | Node::PhpEcho(_) | Node::Doctype(_) | Node::Comment(_) => false,
    }
}

fn is_inline_run_start(node: &Node) -> bool {
    matches!(node, Node::Text(_))
        || matches!(node, Node::PhpEcho(code) if is_inline_echo_code(code))
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
            ..
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
                if is_echo_block_opener(code) || is_echo_block_closer(code) || !is_inline_echo_code(code) {
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
            Node::Element { .. } | Node::PhpBlock(_) | Node::Doctype(_) | Node::Comment(_) => break,
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
    matches!(node, Node::PhpEcho(code) if is_inline_echo_code(code))
        || matches!(node, Node::PhpBlock(code) if is_single_echo_block(code))
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
            Node::Text(_)
            | Node::Element { .. }
            | Node::PhpBlock(_)
            | Node::PhpEcho(_)
            | Node::Doctype(_)
            | Node::Comment(_) => break,
        }
    }
    merged_any.then_some((merged, j))
}

fn try_merge_switch_case_blocks(nodes: &[Node], start: usize, code: &str) -> Option<(String, usize)> {
    let current = code.trim();
    if !current.to_lowercase().starts_with("switch") || !is_php_block_opener(current) {
        return None;
    }
    match nodes.get(start + 1) {
        Some(Node::PhpBlock(next)) if is_switch_case_peer(next) => {
            Some((format!("{current}\n{}", next.trim()), start + 2))
        }
        Some(
            Node::Element { .. }
            | Node::Text(_)
            | Node::PhpBlock(_)
            | Node::PhpEcho(_)
            | Node::Doctype(_)
            | Node::Comment(_),
        )
        | None => None,
    }
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
            el @ Node::Element { .. } => {
                self.emit_element(el, (state.depth, output));
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
                if let Some((merged, j)) = try_merge_switch_case_blocks(nodes, i, code) {
                    self.emit_php_block(&merged, pad, state, output);
                    return j;
                }
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
            Node::Comment(s) => output.push_str(&format!("{pad}{}\n", render_comment(s))),
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
            Node::Comment(s) => output.push_str(&format!("{pad}{}\n", render_comment(s))),
            Node::Element {
                name,
                attributes,
                children,
                ..
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
