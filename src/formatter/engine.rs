use super::php::{format_php_code, join_php_lines, split_by_args, split_by_chain, split_by_commas, split_by_concat};
use crate::parser::ast::Node;
use crate::parser::lexer::Attribute;

const INDENT: &str = "    ";
const MAX_LINE_LENGTH: usize = 120;

const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source", "track", "wbr",
];

const RAW_TEXT_ELEMENTS: &[&str] = &["script", "style", "textarea"];

fn contains_outside_strings(code: &str, needle: &str) -> bool {
    let bytes = code.as_bytes();
    let needle_bytes = needle.as_bytes();
    let len = bytes.len();
    let nlen = needle_bytes.len();
    let mut i = 0;
    while i < len {
        if bytes[i] == b'\'' || bytes[i] == b'"' {
            let q = bytes[i];
            i += 1;
            while i < len && bytes[i] != q {
                if bytes[i] == b'\\' {
                    i += 1;
                }
                i += 1;
            }
            if i < len {
                i += 1;
            }
            continue;
        }
        if i + nlen <= len && &bytes[i..i + nlen] == needle_bytes {
            return true;
        }
        i += 1;
    }
    false
}

fn has_switch_case(code: &str) -> bool {
    let lower = code.to_lowercase();
    (lower.contains("switch") || contains_outside_strings(&lower, "break"))
        && (lower.contains("case ") || lower.contains("default:"))
}

fn is_php_block_opener(code: &str) -> bool {
    let trimmed = code.trim();
    trimmed.ends_with(':') || trimmed.ends_with('{') || trimmed.contains("::begin(")
}

fn is_php_block_closer(code: &str) -> bool {
    let lower = code.trim().to_lowercase();
    lower.starts_with("endif")
        || lower.starts_with("endforeach")
        || lower.starts_with("endfor")
        || lower.starts_with("endwhile")
        || lower.starts_with("endswitch")
        || lower.starts_with("else")
        || lower.starts_with("elseif")
        || lower.starts_with('}')
        || contains_outside_strings(&lower, "break;")
        || lower.starts_with("case ")
        || lower.starts_with("default:")
        || lower.contains("::end(")
}

fn is_switch_case_peer(code: &str) -> bool {
    let lower = code.trim().to_lowercase();
    lower.starts_with("case ") || lower.starts_with("default:")
}

fn is_echo_block_opener(code: &str) -> bool {
    let trimmed = code.trim().to_lowercase();
    trimmed.contains("begintag(") || trimmed.contains("::begin(")
}

fn is_echo_block_closer(code: &str) -> bool {
    let trimmed = code.trim().to_lowercase();
    trimmed.contains("endtag(") || trimmed.contains("::end(")
}

fn is_header_php_block(code: &str) -> bool {
    code.lines().any(|line| {
        let t = line.trim();
        t.starts_with("use ") || t.starts_with("declare(")
    })
}

fn split_header_and_opener(code: &str) -> Option<(String, String)> {
    if !is_php_block_opener(code) {
        return None;
    }
    let normalized = normalize_statements(code);
    let lines: Vec<&str> = normalized.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.len() < 2 {
        return None;
    }
    let last = lines.last()?.trim();
    let lower = last.to_lowercase();
    let is_opener = (lower.starts_with("if ")
        || lower.starts_with("if(")
        || lower.starts_with("foreach ")
        || lower.starts_with("foreach(")
        || lower.starts_with("for ")
        || lower.starts_with("for(")
        || lower.starts_with("while ")
        || lower.starts_with("while(")
        || lower.starts_with("switch "))
        && last.ends_with(':');
    if !is_opener {
        return None;
    }
    let opener = last.to_string();
    let orig_lines: Vec<&str> = code.lines().collect();
    if orig_lines.len() > 1 {
        let mut end = orig_lines.len().saturating_sub(1);
        while end > 0 && orig_lines[end].trim().is_empty() {
            end -= 1;
        }
        let mut start = 0;
        while start < end && orig_lines[start].trim().is_empty() {
            start += 1;
        }
        if start < end {
            let header = orig_lines[start..end].join("\n");
            return Some((header, opener));
        }
    }
    let header_lines: Vec<&str> = lines[..lines.len() - 1].to_vec();
    let header = header_lines.join("\n");
    Some((header, opener))
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

fn is_single_echo_block(code: &str) -> bool {
    let trimmed = code.trim();
    trimmed.starts_with("echo ") && !trimmed.contains('\n') && trimmed.matches(';').count() <= 1
}

fn expand_single_line_docblock(code: &str) -> Option<String> {
    let trimmed = code.trim();
    if trimmed.contains('\n') || !trimmed.starts_with("/**") || !trimmed.ends_with("*/") {
        return None;
    }

    let mut body = trimmed.strip_prefix("/**").and_then(|s| s.strip_suffix("*/"))?.trim();
    if let Some(rest) = body.strip_prefix('*') {
        body = rest.trim_start();
    }

    if body.is_empty() {
        Some("/**\n */".to_string())
    } else {
        Some(format!("/**\n * {body}\n */"))
    }
}

fn extract_docblock_body(code: &str) -> Option<String> {
    let trimmed = code.trim();
    if trimmed.contains('\n') || !trimmed.starts_with("/**") || !trimmed.ends_with("*/") {
        return None;
    }
    let mut body = trimmed.strip_prefix("/**").and_then(|s| s.strip_suffix("*/"))?.trim();
    if let Some(rest) = body.strip_prefix('*') {
        body = rest.trim_start();
    }
    if body.is_empty() {
        return None;
    }
    Some(body.to_string())
}

fn merge_docblock_bodies(bodies: &[String]) -> String {
    let mut result = String::from("/**");
    for body in bodies {
        if body.is_empty() {
            result.push_str("\n *");
        } else {
            result.push_str(&format!("\n * {body}"));
        }
    }
    result.push_str("\n */");
    result
}

fn merge_descriptions_and_vars(descriptions: &[String], vars: &[String]) -> Vec<String> {
    let mut all_bodies: Vec<String> = Vec::new();
    all_bodies.extend_from_slice(descriptions);
    if !descriptions.is_empty() && !vars.is_empty() {
        all_bodies.push(String::new());
    }
    all_bodies.extend_from_slice(vars);
    all_bodies
}

fn flush_docblocks(bodies: &[String], pad: &str, depth: &mut i32, result: &mut String) {
    let merged = if bodies.len() == 1 {
        format!("/**\n * {}\n */", bodies[0])
    } else {
        merge_docblock_bodies(bodies)
    };
    for doc_line in merged.lines() {
        emit_reindented_line(doc_line, pad, depth, result);
    }
}

fn is_docblock_only(code: &str) -> bool {
    let trimmed = code.trim();
    if trimmed.is_empty() {
        return false;
    }

    if trimmed.starts_with("/**") && trimmed.ends_with("*/") && !trimmed.contains('\n') {
        return true;
    }

    let lines: Vec<&str> = trimmed.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    if lines.len() < 2 {
        return false;
    }
    if lines.first().copied() != Some("/**") || lines.last().copied() != Some("*/") {
        return false;
    }

    lines[1..lines.len() - 1].iter().all(|line| line.starts_with('*'))
}

fn emit_docblock_php(code: &str, pad: &str, output: &mut String) {
    let docblock = expand_single_line_docblock(code).unwrap_or_else(|| code.trim().to_string());
    output.push_str(&format!("{pad}<?php\n"));
    for line in docblock.lines() {
        output.push_str(pad);
        output.push_str(line.trim_end());
        output.push('\n');
    }
    output.push_str(&format!("{pad}?>\n"));
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

fn skip_string_literal(chars: &[char], start: usize, result: &mut String) -> usize {
    let quote = chars[start];
    let len = chars.len();
    result.push(quote);
    let mut i = start + 1;
    while i < len && chars[i] != quote {
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
    i
}

fn handle_comment_boundaries(chars: &[char], i: usize, ch: char, result: &mut String) {
    if ch == '/'
        && i + 1 < chars.len()
        && chars[i + 1] == '*'
        && i + 2 < chars.len()
        && chars[i + 2] == '*'
        && !result.ends_with('\n')
        && result.len() > 1
    {
        let last = result.pop().unwrap_or_default();
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
        if i + 1 < chars.len() && chars[i + 1] != '\n' {
            result.push('\n');
        }
    }
    if ch == '*'
        && i + 1 < chars.len()
        && chars[i + 1] == ' '
        && i + 2 < chars.len()
        && chars[i + 2] == '@'
        && !result.ends_with('\n')
        && !result.ends_with('/')
        && !result.ends_with("/**")
    {
        result.pop();
        result.push('\n');
        result.push(ch);
    }
}

fn normalize_statements(code: &str) -> String {
    let mut result = String::from("\n");
    let chars: Vec<char> = code.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut paren_depth: i32 = 0;

    while i < len {
        let ch = chars[i];
        if ch == '\'' || ch == '"' {
            i = skip_string_literal(&chars, i, &mut result);
            continue;
        }
        if ch == '(' {
            paren_depth += 1;
        } else if ch == ')' {
            paren_depth -= 1;
        }

        if paren_depth <= 0
            && !result.ends_with('\n')
            && !result.trim_end().is_empty()
            && (matches_keyword_at(&chars, i, "case ") || matches_keyword_at(&chars, i, "default:"))
        {
            result.push('\n');
        }

        result.push(ch);
        if ch == ';' && paren_depth <= 0 && i + 1 < len && chars[i + 1] != '\n' {
            result.push('\n');
        }

        handle_comment_boundaries(&chars, i, ch, &mut result);
        i += 1;
    }

    result
}

fn matches_keyword_at(chars: &[char], pos: usize, keyword: &str) -> bool {
    let kw: Vec<char> = keyword.chars().collect();
    if pos + kw.len() > chars.len() {
        return false;
    }
    for (j, kc) in kw.iter().enumerate() {
        if chars[pos + j] != *kc {
            return false;
        }
    }
    if pos > 0 && chars[pos - 1].is_alphanumeric() {
        return false;
    }
    true
}

fn emit_reindented_line(formatted: &str, pad: &str, depth: &mut i32, result: &mut String) {
    let leading = count_leading_closers(formatted) as i32;
    let is_ternary_cont = formatted.starts_with("? ") || formatted.starts_with(": ");
    let extra = i32::from(is_ternary_cont);
    let write_depth = (*depth - leading + extra).max(0) as usize;
    let inner_pad = INDENT.repeat(write_depth);
    let base_pad = format!("{pad}{inner_pad}");
    if let Some(split) = try_split_long_line(formatted, &base_pad) {
        result.push_str(&split);
    } else if formatted.starts_with('*') {
        result.push_str(&format!("{pad}{inner_pad} {formatted}\n"));
    } else {
        result.push_str(&format!("{pad}{inner_pad}{formatted}\n"));
    }
    let (openers, closers) = count_brackets(formatted);
    let net = openers as i32 - closers as i32;
    *depth += net.min(1);
    *depth = (*depth).max(0);
}

fn join_ternary_lines(code: &str) -> String {
    let lines: Vec<&str> = code.lines().collect();
    let mut result: Vec<String> = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim();
        let ends_with_ternary = trimmed.ends_with(" ?") || (trimmed.ends_with('?') && !trimmed.ends_with("?>"));
        let next_is_continuation =
            i + 1 < lines.len() && (lines[i + 1].trim().starts_with("? ") || lines[i + 1].trim().starts_with(": "));
        let should_join = ends_with_ternary || next_is_continuation;

        if !should_join {
            result.push(lines[i].to_string());
            i += 1;
            continue;
        }

        let mut joined = trimmed.to_string();
        i += 1;
        while i < lines.len() {
            let next = lines[i].trim();
            if next.is_empty() {
                break;
            }
            joined.push(' ');
            joined.push_str(next);
            i += 1;
            if next.contains(';') {
                break;
            }
        }
        result.push(joined);
    }

    result.join("\n")
}

fn count_unescaped_quotes(line: &str, quote: char) -> usize {
    let mut count = 0;
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\' {
            i += 2;
            continue;
        }
        if chars[i] == quote {
            count += 1;
        }
        i += 1;
    }
    count
}

fn has_unclosed_string(line: &str) -> bool {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    let mut in_str: Option<char> = None;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '\\' && in_str.is_some() {
            i += 2;
            continue;
        }
        match in_str {
            Some(q) if ch == q => in_str = None,
            Some(_) => {}
            None if ch == '\'' || ch == '"' => in_str = Some(ch),
            _ => {}
        }
        i += 1;
    }
    in_str.is_some()
}

fn detect_open_quote(line: &str) -> Option<char> {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    let mut in_str: Option<char> = None;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '\\' && in_str.is_some() {
            i += 2;
            continue;
        }
        match in_str {
            Some(q) if ch == q => in_str = None,
            Some(_) => {}
            None if ch == '\'' || ch == '"' => in_str = Some(ch),
            _ => {}
        }
        i += 1;
    }
    in_str
}

fn detect_heredoc(line: &str) -> Option<String> {
    let pos = line.find("<<<")?;
    let after = line[pos + 3..].trim();
    if after.is_empty() {
        return None;
    }
    let marker = after.trim_matches('\'').trim_matches('"');
    let marker = marker.trim_end_matches(',');
    if marker.chars().all(|c| c.is_alphanumeric() || c == '_') && !marker.is_empty() {
        Some(marker.to_string())
    } else {
        None
    }
}

fn reindent_php_block(code: &str, pad: &str) -> String {
    let needs_normalize = !code.contains('\n') && (code.contains(';') || has_switch_case(code));
    let code = if needs_normalize {
        normalize_statements(code)
    } else {
        join_ternary_lines(code)
    };
    let mut result = String::new();
    let mut depth: i32 = 0;
    let mut prev_blank = false;
    let mut first_content = true;
    let mut prev_was_doc_close = false;
    let mut prev_was_use = false;
    let mut prev_was_declare = false;
    let is_header = is_header_php_block(&code);
    let mut in_string: Option<char> = None;
    let mut heredoc_marker: Option<String> = None;
    let mut pending_docblocks: Vec<String> = Vec::new();
    let mut pending_descriptions: Vec<String> = Vec::new();
    let mut deferred_lines: Vec<String> = Vec::new();
    let mut pending_count_at_defer: usize = 0;
    let mut in_docblock = false;
    let mut docblock_bodies: Vec<String> = Vec::new();

    for line in code.lines() {
        if let Some(ref marker) = heredoc_marker {
            result.push_str(line);
            result.push('\n');
            let closing = line.trim().trim_end_matches(';');
            if closing == marker.as_str() {
                let m = marker.clone();
                heredoc_marker = None;
                let after_marker = line.trim().strip_prefix(m.as_str()).unwrap_or("");
                let (o, c) = count_brackets(after_marker);
                depth += o as i32 - c as i32;
                depth = depth.max(0);
            }
            continue;
        }
        if let Some(quote) = in_string {
            result.push_str(line);
            result.push('\n');
            if count_unescaped_quotes(line, quote) % 2 == 1 {
                in_string = None;
                if let Some(pos) = line.rfind(quote) {
                    let after_quote = &line[pos + 1..];
                    let (o, c) = count_brackets(after_quote);
                    depth += o as i32 - c as i32;
                    depth = depth.max(0);
                }
            }
            continue;
        }
        let trimmed = line.trim();

        if in_docblock {
            if trimmed == "*/" {
                in_docblock = false;
                let all_var = !docblock_bodies.is_empty() && docblock_bodies.iter().all(|b| b.starts_with("@var "));
                if all_var {
                    pending_docblocks.append(&mut docblock_bodies);
                } else if is_header {
                    pending_descriptions.append(&mut docblock_bodies);
                } else {
                    if !pending_docblocks.is_empty() || !pending_descriptions.is_empty() {
                        flush_docblocks(
                            &merge_descriptions_and_vars(&pending_descriptions, &pending_docblocks),
                            pad,
                            &mut depth,
                            &mut result,
                        );
                        pending_docblocks.clear();
                        pending_descriptions.clear();
                    }
                    emit_reindented_line("/**", pad, &mut depth, &mut result);
                    for body in &docblock_bodies {
                        emit_reindented_line(&format!("* {body}"), pad, &mut depth, &mut result);
                    }
                    emit_reindented_line("*/", pad, &mut depth, &mut result);
                    docblock_bodies.clear();
                    prev_was_doc_close = true;
                }
            } else if let Some(body) = trimmed.strip_prefix("* ") {
                docblock_bodies.push(body.to_string());
            } else if trimmed == "*" {
                docblock_bodies.push(String::new());
            }
            continue;
        }

        if trimmed.is_empty() {
            if !prev_blank && !first_content {
                if pending_docblocks.is_empty() && !in_docblock {
                    result.push('\n');
                }
                prev_blank = true;
            }
            continue;
        }
        if first_content && !prev_blank && is_header {
            result.push('\n');
        }
        first_content = false;
        let is_use_import = trimmed.starts_with("use ");
        let is_declare = trimmed.starts_with("declare(");

        let has_pending = !pending_docblocks.is_empty() || !pending_descriptions.is_empty();
        if has_pending && extract_docblock_body(trimmed).is_none() && trimmed != "/**" && !is_use_import && !is_declare
        {
            let pending_grew = (pending_docblocks.len() + pending_descriptions.len()) > pending_count_at_defer;
            if pending_grew && !deferred_lines.is_empty() {
                for dl in &deferred_lines {
                    emit_reindented_line(dl, pad, &mut depth, &mut result);
                }
                result.push('\n');
                deferred_lines.clear();
            }
            flush_docblocks(
                &merge_descriptions_and_vars(&pending_descriptions, &pending_docblocks),
                pad,
                &mut depth,
                &mut result,
            );
            pending_docblocks.clear();
            pending_descriptions.clear();
            prev_was_doc_close = true;
            prev_blank = false;
            if !pending_grew && !deferred_lines.is_empty() {
                result.push('\n');
                for dl in &deferred_lines {
                    emit_reindented_line(dl, pad, &mut depth, &mut result);
                }
                deferred_lines.clear();
                prev_was_doc_close = false;
                prev_was_use = true;
            }
        } else if has_pending && (is_use_import || is_declare) {
            if deferred_lines.is_empty() {
                pending_count_at_defer = pending_docblocks.len() + pending_descriptions.len();
            }
            deferred_lines.push(trimmed.to_string());
            prev_was_use = is_use_import;
            prev_was_declare = is_declare;
            continue;
        }

        if (prev_was_use && !is_use_import && !prev_blank) || (prev_was_declare && !is_declare && !prev_blank) {
            result.push('\n');
        }
        if prev_was_doc_close && !prev_blank {
            result.push('\n');
        }
        prev_blank = false;
        prev_was_doc_close = trimmed == "*/";
        prev_was_use = is_use_import;
        prev_was_declare = is_declare;

        if let Some(body) = extract_docblock_body(trimmed) {
            pending_docblocks.push(body);
            prev_was_use = false;
            prev_was_declare = false;
            continue;
        }

        if trimmed == "/**" {
            in_docblock = true;
            docblock_bodies.clear();
            continue;
        }

        let formatted = format_php_code(trimmed);
        emit_reindented_line(&formatted, pad, &mut depth, &mut result);
        if let Some(marker) = detect_heredoc(trimmed) {
            heredoc_marker = Some(marker);
        } else if has_unclosed_string(trimmed) {
            in_string = detect_open_quote(trimmed);
        }
    }

    if !pending_docblocks.is_empty() || !pending_descriptions.is_empty() {
        let pending_grew = (pending_docblocks.len() + pending_descriptions.len()) > pending_count_at_defer;
        if pending_grew && !deferred_lines.is_empty() {
            for dl in &deferred_lines {
                emit_reindented_line(dl, pad, &mut depth, &mut result);
            }
            result.push('\n');
            deferred_lines.clear();
        }
        flush_docblocks(
            &merge_descriptions_and_vars(&pending_descriptions, &pending_docblocks),
            pad,
            &mut depth,
            &mut result,
        );
        if !deferred_lines.is_empty() {
            result.push('\n');
            for dl in &deferred_lines {
                emit_reindented_line(dl, pad, &mut depth, &mut result);
            }
        }
    }

    result.trim_end_matches('\n').to_string() + "\n"
}

fn find_matching_close(chars: &[char], open_pos: usize) -> Option<usize> {
    let len = chars.len();
    let mut depth = 0i32;
    let mut i = open_pos;
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
            depth += 1;
        } else if matches!(ch, ')' | ']' | '}') {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn try_split_long_line(formatted: &str, base_pad: &str) -> Option<String> {
    if base_pad.len() + formatted.len() <= MAX_LINE_LENGTH {
        return None;
    }

    if let Some((q_pos, c_pos)) = find_ternary_positions(formatted) {
        let condition = formatted[..q_pos].trim_end();
        let true_val = formatted[q_pos + 1..c_pos].trim();
        let false_val = formatted[c_pos + 1..].trim();
        let inner_pad = format!("{base_pad}{INDENT}");
        let mut result = format!("{base_pad}{condition}\n");
        append_ternary_value(&mut result, '?', true_val, &inner_pad);
        append_ternary_value(&mut result, ':', false_val, &inner_pad);
        return Some(result);
    }

    if let Some((prefix, args, suffix)) = split_by_args(formatted) {
        return Some(build_split(&prefix, &args, &suffix, base_pad));
    }

    let chars: Vec<char> = formatted.chars().collect();
    let open_pos = chars.iter().position(|&c| c == '(')?;
    let close_pos = find_matching_close(&chars, open_pos)?;

    let inner: String = chars[open_pos + 1..close_pos].iter().collect();
    let inner = inner.trim();

    if let Some(array_inner) = inner.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
        let items = split_by_commas(array_inner);
        if items.len() > 1 {
            let prefix: String = chars[..=open_pos].iter().collect();
            let suffix: String = chars[close_pos..].iter().collect();
            let new_prefix = format!("{prefix}[");
            let new_suffix = format!("]{suffix}");
            return Some(build_split(&new_prefix, &items, &new_suffix, base_pad));
        }
    }

    if let Some(expanded) = expand_nested_array(formatted, base_pad) {
        return Some(expanded);
    }

    None
}

fn append_ternary_value(result: &mut String, marker: char, value: &str, line_pad: &str) {
    let single_len = line_pad.len() + 2 + value.len();
    if single_len <= MAX_LINE_LENGTH {
        result.push_str(&format!("{line_pad}{marker} {value}\n"));
        return;
    }

    if let Some(split) = try_split_long_line(value, line_pad) {
        let mut lines = split.lines();
        if let Some(first) = lines.next() {
            let first = first.strip_prefix(line_pad).unwrap_or(first).trim_start();
            result.push_str(&format!("{line_pad}{marker} {first}\n"));
            for line in lines {
                if !line.trim().is_empty() {
                    result.push_str(line);
                    result.push('\n');
                }
            }
            return;
        }
    }

    result.push_str(&format!("{line_pad}{marker} {value}\n"));
}

fn build_split(prefix: &str, args: &[String], suffix: &str, pad: &str) -> String {
    let inner_pad = format!("{pad}{INDENT}");
    let mut result = format!("{pad}{prefix}\n");
    for arg in args {
        let line_len = inner_pad.len() + arg.len() + 1;
        if line_len > MAX_LINE_LENGTH {
            if let Some(expanded) = expand_nested_array(arg, &inner_pad) {
                result.push_str(&expanded);
                continue;
            }
            if let Some(expanded) = expand_bare_array(arg, &inner_pad) {
                result.push_str(&expanded);
                continue;
            }
            if let Some(expanded) = expand_inline_closure(arg, &inner_pad) {
                result.push_str(&expanded);
                continue;
            }
            if let Some(split) = try_split_long_line(arg, &inner_pad) {
                let trimmed = split.trim_end_matches('\n');
                result.push_str(trimmed);
                result.push_str(",\n");
                continue;
            }
        }
        result.push_str(&format!("{inner_pad}{arg},\n"));
    }
    result.push_str(&format!("{pad}{suffix}\n"));
    result
}

fn expand_bare_array(arg: &str, pad: &str) -> Option<String> {
    let trimmed = arg.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return None;
    }
    let inner = &trimmed[1..trimmed.len() - 1];
    let items = split_by_commas(inner);
    if items.len() <= 1 {
        if items.len() == 1 {
            let item = &items[0];
            let nested_pad = format!("{pad}{INDENT}");
            let item_line_len = nested_pad.len() + item.len() + 1;
            if item_line_len > MAX_LINE_LENGTH {
                let mut result = format!("{pad}[\n");
                if let Some(split) = try_split_long_line(item, &nested_pad) {
                    let trimmed = split.trim_end_matches('\n');
                    result.push_str(trimmed);
                    result.push_str(",\n");
                } else {
                    result.push_str(&format!("{nested_pad}{item},\n"));
                }
                result.push_str(&format!("{pad}],\n"));
                return Some(result);
            }
        }
        return None;
    }
    let nested_pad = format!("{pad}{INDENT}");
    let mut result = format!("{pad}[\n");
    for item in &items {
        let item_line_len = nested_pad.len() + item.len() + 1;
        if item_line_len > MAX_LINE_LENGTH {
            if let Some(expanded) = expand_nested_array(item, &nested_pad) {
                result.push_str(&expanded);
                continue;
            }
        }
        if item.starts_with('[') && item.ends_with(']') {
            let sub_inner = &item[1..item.len() - 1];
            let sub_items = split_by_commas(sub_inner);
            if sub_items.len() > 1 {
                let deeper_pad = format!("{nested_pad}{INDENT}");
                result.push_str(&format!("{nested_pad}[\n"));
                for sub in &sub_items {
                    result.push_str(&format!("{deeper_pad}{sub},\n"));
                }
                result.push_str(&format!("{nested_pad}],\n"));
                continue;
            }
        }
        result.push_str(&format!("{nested_pad}{item},\n"));
    }
    result.push_str(&format!("{pad}],\n"));
    Some(result)
}

fn expand_bare_sub_array(item: &str, pad: &str) -> Option<String> {
    if !item.starts_with('[') || !item.ends_with(']') {
        return None;
    }
    let sub_inner = &item[1..item.len() - 1];
    let sub_items = split_by_commas(sub_inner);
    if sub_items.len() <= 1 {
        return None;
    }
    let deeper_pad = format!("{pad}{INDENT}");
    let mut result = format!("{pad}[\n");
    for sub in &sub_items {
        let sub_line_len = deeper_pad.len() + sub.len() + 1;
        if sub_line_len > MAX_LINE_LENGTH {
            if let Some(expanded) = expand_nested_array(sub, &deeper_pad) {
                result.push_str(&expanded);
                continue;
            }
            if let Some(expanded) = expand_inline_closure(sub, &deeper_pad) {
                result.push_str(&expanded);
                continue;
            }
            if let Some(split) = try_split_long_line(sub, &deeper_pad) {
                let trimmed = split.trim_end_matches('\n');
                result.push_str(trimmed);
                result.push_str(",\n");
                continue;
            }
        }
        result.push_str(&format!("{deeper_pad}{sub},\n"));
    }
    result.push_str(&format!("{pad}],\n"));
    Some(result)
}

fn find_closure_body(code: &str) -> Option<(usize, usize)> {
    let chars: Vec<char> = code.chars().collect();
    let len = chars.len();
    let mut i = 0;
    while i < len {
        if chars[i] == 'f'
            && i + 8 < len
            && chars[i + 1] == 'u'
            && chars[i + 2] == 'n'
            && chars[i + 3] == 'c'
            && chars[i + 4] == 't'
            && chars[i + 5] == 'i'
            && chars[i + 6] == 'o'
            && chars[i + 7] == 'n'
        {
            if i > 0 && chars[i - 1].is_alphanumeric() {
                i += 1;
                continue;
            }
            let mut j = i + 8;
            while j < len && chars[j] != '{' {
                j += 1;
            }
            if j < len {
                if let Some(close) = find_matching_close(&chars, j) {
                    return Some((j, close));
                }
            }
        }
        i += 1;
    }
    None
}

fn normalize_closure_body(body: &str) -> Vec<String> {
    let chars: Vec<char> = body.chars().collect();
    let len = chars.len();
    let mut statements: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut i = 0;
    let mut brace_depth: i32 = 0;
    let mut paren_depth: i32 = 0;

    while i < len {
        let ch = chars[i];
        if ch == '\'' || ch == '"' {
            current.push(ch);
            i += 1;
            while i < len && chars[i] != ch {
                if chars[i] == '\\' {
                    current.push(chars[i]);
                    i += 1;
                    if i < len {
                        current.push(chars[i]);
                        i += 1;
                    }
                    continue;
                }
                current.push(chars[i]);
                i += 1;
            }
            if i < len {
                current.push(chars[i]);
                i += 1;
            }
            continue;
        }
        if ch == '(' {
            paren_depth += 1;
        } else if ch == ')' {
            paren_depth -= 1;
        } else if ch == '{' {
            brace_depth += 1;
        } else if ch == '}' && brace_depth > 0 {
            brace_depth -= 1;
            current.push(ch);
            if brace_depth == 0 {
                let trimmed = current.trim().to_string();
                if !trimmed.is_empty() {
                    statements.push(trimmed);
                }
                current.clear();
                i += 1;
                continue;
            }
            i += 1;
            continue;
        }
        current.push(ch);
        if ch == ';' && brace_depth == 0 && paren_depth <= 0 {
            let trimmed = current.trim().to_string();
            if !trimmed.is_empty() {
                statements.push(trimmed);
            }
            current.clear();
        }
        i += 1;
    }
    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        statements.push(trimmed);
    }
    statements
}

fn find_brace_block(code: &str) -> Option<(usize, usize)> {
    let chars: Vec<char> = code.chars().collect();
    let len = chars.len();
    let mut i = 0;
    while i < len {
        if chars[i] == '\'' || chars[i] == '"' {
            let q = chars[i];
            i += 1;
            while i < len && chars[i] != q {
                if chars[i] == '\\' {
                    i += 1;
                }
                i += 1;
            }
            if i < len {
                i += 1;
            }
            continue;
        }
        if chars[i] == '{' {
            if let Some(close) = find_matching_close(&chars, i) {
                return Some((i, close));
            }
        }
        i += 1;
    }
    None
}

fn expand_brace_block(stmt: &str, pad: &str) -> Option<String> {
    let (open, close) = find_brace_block(stmt)?;
    let chars: Vec<char> = stmt.chars().collect();
    let body: String = chars[open + 1..close].iter().collect();
    let body = body.trim();
    if body.is_empty() {
        return None;
    }
    let header: String = chars[..open].iter().collect();
    let header = header.trim_end();
    let after: String = chars[close + 1..].iter().collect();
    let after = after.trim();
    let inner_pad = format!("{pad}{INDENT}");
    let body_stmts = normalize_closure_body(body);
    if body_stmts.is_empty() {
        return None;
    }
    let mut result = format!("{pad}{header} {{\n");
    for s in &body_stmts {
        let line_len = inner_pad.len() + s.len();
        if line_len > MAX_LINE_LENGTH {
            if let Some(split) = try_split_long_line(s, &inner_pad) {
                result.push_str(&split);
                continue;
            }
        }
        result.push_str(&format!("{inner_pad}{s}\n"));
    }
    if after.is_empty() {
        result.push_str(&format!("{pad}}}\n"));
    } else {
        result.push_str(&format!("{pad}}} {after}\n"));
    }
    Some(result)
}

fn expand_inline_closure(arg: &str, pad: &str) -> Option<String> {
    let (open_brace, close_brace) = find_closure_body(arg)?;
    let chars: Vec<char> = arg.chars().collect();
    let body: String = chars[open_brace + 1..close_brace].iter().collect();
    let stmts = normalize_closure_body(&body);
    if stmts.len() <= 1 {
        return None;
    }
    let header: String = chars[..open_brace].iter().collect();
    let header = header.trim_end();
    let after_close: String = chars[close_brace + 1..].iter().collect();
    let after_close = after_close.trim_start();
    let body_pad = format!("{pad}{INDENT}");
    let mut result = format!("{pad}{header} {{\n");
    for stmt in &stmts {
        if let Some(expanded) = expand_brace_block(stmt, &body_pad) {
            result.push_str(&expanded);
            continue;
        }
        let line_len = body_pad.len() + stmt.len();
        if line_len > MAX_LINE_LENGTH {
            if let Some(split) = try_split_long_line(stmt, &body_pad) {
                result.push_str(&split);
                continue;
            }
        }
        result.push_str(&format!("{body_pad}{stmt}\n"));
    }
    if after_close.is_empty() {
        result.push_str(&format!("{pad}}},\n"));
    } else {
        result.push_str(&format!("{pad}}}{after_close}\n"));
    }
    Some(result)
}

fn find_array_arrow(arg: &str) -> Option<(usize, usize)> {
    let bytes = arg.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    if i < len && (bytes[i] == b'\'' || bytes[i] == b'"') {
        let quote = bytes[i];
        i += 1;
        while i < len && bytes[i] != quote {
            if bytes[i] == b'\\' {
                i += 1;
            }
            i += 1;
        }
        if i < len {
            i += 1;
        }
    }
    let arrow_pos = arg[i..].find("=>")?;
    Some((i, arrow_pos))
}

fn expand_nested_array(arg: &str, pad: &str) -> Option<String> {
    let (skip, arrow_pos) = find_array_arrow(arg)?;
    let value = arg[skip + arrow_pos + 2..].trim();
    if !value.starts_with('[') || !value.ends_with(']') {
        return None;
    }
    let inner = &value[1..value.len() - 1];
    let items = split_by_commas(inner);
    if items.len() <= 1 {
        return None;
    }
    let key = &arg[..skip + arrow_pos + 2];
    let nested_pad = format!("{pad}{INDENT}");
    let mut result = format!("{pad}{key} [\n");
    for item in &items {
        if nested_pad.len() + item.len() + 1 > MAX_LINE_LENGTH {
            if let Some(expanded) = expand_nested_array(item, &nested_pad) {
                result.push_str(&expanded);
                continue;
            }
            if let Some(bare) = expand_bare_sub_array(item, &nested_pad) {
                result.push_str(&bare);
                continue;
            }
            if let Some(expanded) = expand_inline_closure(item, &nested_pad) {
                result.push_str(&expanded);
                continue;
            }
            if let Some(split) = try_split_long_line(item, &nested_pad) {
                let trimmed = split.trim_end_matches('\n');
                result.push_str(trimmed);
                result.push_str(",\n");
                continue;
            }
        }
        if let Some(bare) = expand_bare_sub_array(item, &nested_pad) {
            result.push_str(&bare);
            continue;
        }
        result.push_str(&format!("{nested_pad}{item},\n"));
    }
    result.push_str(&format!("{pad}],\n"));
    Some(result)
}

fn format_echo_chain(parts: &[String], pad: &str) -> String {
    let chain_pad = format!("{pad}{INDENT}");
    let mut result = format!("{pad}<?= {}{}", parts[0], parts[1]);
    for part in &parts[2..] {
        let part_line_len = chain_pad.len() + part.len();
        if part_line_len > MAX_LINE_LENGTH {
            if let Some(split) = try_split_long_line(part, &chain_pad) {
                let split_content = split.trim_start().trim_end_matches('\n');
                result.push_str(&format!("\n{chain_pad}{split_content}"));
                continue;
            }
        }
        result.push_str(&format!("\n{chain_pad}{part}"));
    }
    result.push_str(" ?>\n");
    result
}

fn format_echo_concat(parts: &[String], pad: &str) -> String {
    let concat_pad = format!("{pad}{INDENT}");
    let mut result = format!("{pad}<?= {}", parts[0]);
    for part in &parts[1..] {
        result.push_str(&format!("\n{concat_pad}. {part}"));
    }
    result.push_str(" ?>\n");
    result
}

fn format_echo_array_split(prefix: &str, args: &[String], suffix: &str, pad: &str) -> Option<String> {
    let last = &args[args.len() - 1];
    if !last.starts_with('[') || !last.ends_with(']') {
        return None;
    }
    let inline_parts: Vec<&str> = args[..args.len() - 1].iter().map(|s| s.as_str()).collect();
    let inline_joined = inline_parts.join(", ");
    let inline_prefix = format!("{pad}<?= {prefix}{inline_joined}, [");
    if inline_prefix.len() > MAX_LINE_LENGTH {
        return None;
    }
    let inner = &last[1..last.len() - 1];
    let items = split_by_commas(inner);
    if items.len() <= 1 {
        return None;
    }
    let inner_pad = format!("{pad}{INDENT}");
    let mut result = format!("{pad}<?= {prefix}{inline_joined}, [\n");
    for item in &items {
        if inner_pad.len() + item.len() + 1 > MAX_LINE_LENGTH {
            if let Some(expanded) = expand_nested_array(item, &inner_pad) {
                result.push_str(&expanded);
                continue;
            }
            if let Some(bare) = expand_bare_sub_array(item, &inner_pad) {
                result.push_str(&bare);
                continue;
            }
            if let Some(split) = try_split_long_line(item, &inner_pad) {
                let trimmed = split.trim_end_matches('\n');
                result.push_str(trimmed);
                result.push_str(",\n");
                continue;
            }
        }
        result.push_str(&format!("{inner_pad}{item},\n"));
    }
    result.push_str(&format!("{pad}]{suffix} ?>\n"));
    Some(result)
}

fn find_ternary_positions(code: &str) -> Option<(usize, usize)> {
    let bytes = code.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut depth = 0i32;
    let mut question_pos: Option<usize> = None;

    while i < len {
        match bytes[i] {
            b'\'' | b'"' => {
                let q = bytes[i];
                i += 1;
                while i < len && bytes[i] != q {
                    if bytes[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
            }
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth -= 1,
            b'?' if depth == 0 && question_pos.is_none() => {
                if i + 1 < len && bytes[i + 1] == b'>' {
                    i += 2;
                    continue;
                }
                if i + 1 < len && bytes[i + 1] == b'?' {
                    i += 2;
                    continue;
                }
                if i + 1 < len && bytes[i + 1] == b':' {
                    i += 2;
                    continue;
                }
                question_pos = Some(i);
            }
            b':' if depth == 0 && question_pos.is_some() => {
                if i + 1 < len && bytes[i + 1] == b':' {
                    i += 2;
                    continue;
                }
                return Some((question_pos?, i));
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn split_ternary(code: &str, pad: &str) -> Option<String> {
    let (q_pos, c_pos) = find_ternary_positions(code)?;

    let condition = code[..q_pos].trim_end();
    let true_val = code[q_pos + 1..c_pos].trim();
    let false_val = code[c_pos + 1..].trim();

    let inner_pad = format!("{pad}{INDENT}");
    Some(format!(
        "{pad}<?= {condition}\n{inner_pad}? {true_val}\n{inner_pad}: {false_val} ?>\n"
    ))
}

fn format_echo(code: &str, pad: &str) -> String {
    let joined = join_php_lines(code);
    let formatted = format_php_code(&joined);
    let combined = format!("{formatted} ?>");
    let single = format!("{pad}<?= {combined}");

    if single.len() <= MAX_LINE_LENGTH {
        return format!("{single}\n");
    }

    let parts = split_by_chain(&formatted);
    if parts.len() > 2 {
        return format_echo_chain(&parts, pad);
    }

    if let Some(result) = split_ternary(&formatted, pad) {
        return result;
    }

    let concat_parts = split_by_concat(&formatted);
    if concat_parts.len() > 1 {
        return format_echo_concat(&concat_parts, pad);
    }

    if let Some((prefix, args, suffix)) = split_by_args(&formatted) {
        if args.len() >= 2 {
            if let Some(r) = format_echo_array_split(&prefix, &args, &suffix, pad) {
                return r;
            }
        }
        let mut result = format!("{pad}<?= {prefix}\n");
        let inner_pad = format!("{pad}{INDENT}");
        for arg in &args {
            let line_len = inner_pad.len() + arg.len() + 1;
            if line_len > MAX_LINE_LENGTH {
                if let Some(expanded) = expand_nested_array(arg, &inner_pad) {
                    result.push_str(&expanded);
                    continue;
                }
                if let Some(expanded) = expand_bare_array(arg, &inner_pad) {
                    result.push_str(&expanded);
                    continue;
                }
                if let Some(split) = try_split_long_line(arg, &inner_pad) {
                    let trimmed = split.trim_end_matches('\n');
                    result.push_str(trimmed);
                    result.push_str(",\n");
                    continue;
                }
            }
            result.push_str(&format!("{inner_pad}{arg},\n"));
        }
        result.push_str(&format!("{pad}{suffix} ?>\n"));
        return result;
    }

    if let Some(split) = try_split_long_line(&formatted, pad) {
        let trimmed = split.trim_start().trim_end_matches('\n');
        return format!("{pad}<?= {trimmed} ?>\n");
    }

    format!("{single}\n")
}

fn count_semicolons_outside_parens(code: &str) -> usize {
    let mut count = 0;
    let mut depth = 0i32;
    let mut in_str = false;
    let mut str_char = '"';
    let chars_iter: Vec<char> = code.chars().collect();
    let mut ci = 0;
    while ci < chars_iter.len() {
        let c = chars_iter[ci];
        if in_str {
            if c == '\\' {
                ci += 1;
            } else if c == str_char {
                in_str = false;
            }
        } else if c == '\'' || c == '"' {
            in_str = true;
            str_char = c;
        } else if c == '(' {
            depth += 1;
        } else if c == ')' {
            depth -= 1;
        } else if c == ';' && depth <= 0 {
            count += 1;
        }
        ci += 1;
    }
    count
}

struct PhpDepthState {
    depth: usize,
    switch_stack: Vec<usize>,
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
    } else if contains_outside_strings(&lower, "break;") {
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
    } else if !state.switch_stack.is_empty() && contains_outside_strings(&lower, "break;") {
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
        let normalized = normalize_statements(code);
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
    } else if is_multiline {
        emit_multiline_php(code, pad, &mut state.depth, output);
    } else {
        emit_single_php(code, pad, state, output);
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
