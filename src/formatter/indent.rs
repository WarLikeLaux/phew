use super::Formatter;
use super::docblock::{extract_docblock_body, merge_descriptions_and_vars};
use super::php::format_php_code;

pub fn visual_len(s: &str) -> usize {
    s.chars().count()
}

pub fn contains_outside_strings(code: &str, needle: &str) -> bool {
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

pub fn has_switch_case(code: &str) -> bool {
    let lower = code.to_lowercase();
    (lower.contains("switch") || contains_outside_strings(&lower, "break"))
        && (lower.contains("case ") || lower.contains("default:"))
}

pub fn is_php_block_opener(code: &str) -> bool {
    let trimmed = code.trim();
    trimmed.ends_with(':') || trimmed.ends_with('{') || trimmed.contains("::begin(")
}

pub fn is_php_block_closer(code: &str) -> bool {
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

pub fn is_switch_case_peer(code: &str) -> bool {
    let lower = code.trim().to_lowercase();
    lower.starts_with("case ") || lower.starts_with("default:")
}

pub fn is_header_php_block(code: &str) -> bool {
    let has_by_line = code.lines().any(|line| {
        let t = line.trim();
        t.starts_with("use ") || t.starts_with("declare(")
    });
    if has_by_line {
        return true;
    }
    code.contains(" use ") || code.contains(";use ") || code.starts_with("use ")
}

pub fn split_header_and_opener(code: &str) -> Option<(String, String)> {
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

pub fn count_leading_closers(s: &str) -> usize {
    s.chars().take_while(|c| matches!(c, ')' | ']' | '}')).count()
}

pub fn count_brackets(s: &str) -> (usize, usize) {
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

pub fn skip_string_literal(chars: &[char], start: usize, result: &mut String) -> usize {
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

fn expand_inline_docblock(comment: &str) -> String {
    let trimmed = comment.trim();
    if !trimmed.starts_with("/**") || !trimmed.ends_with("*/") || trimmed.len() < 5 {
        return comment.to_string();
    }
    let inner = &trimmed[3..trimmed.len() - 2].trim();

    if !inner.contains("* @") {
        return comment.to_string();
    }

    let mut bodies: Vec<&str> = Vec::new();
    let mut rest = *inner;
    while let Some(pos) = rest.find("* @") {
        let before = rest[..pos].trim().trim_end_matches('*').trim();
        if !before.is_empty() {
            bodies.push(before);
        }
        rest = &rest[pos + 2..];
    }
    let last = rest.trim().trim_end_matches('*').trim();
    if !last.is_empty() {
        bodies.push(last);
    }

    if bodies.len() <= 1 {
        return comment.to_string();
    }

    let mut result = String::from("/**\n");
    for body in &bodies {
        result.push_str(&format!(" * {body}\n"));
    }
    result.push_str(" */");
    result
}

fn emit_block_comment_break(result: &mut String) {
    if !result.ends_with('\n') && result.trim_end().len() > 1 {
        let last = result.pop().unwrap_or_default();
        if last != ' ' && last != '\n' {
            result.push(last);
        }
        if !result.ends_with('\n') {
            result.push('\n');
        }
    }
}

fn collect_block_comment(chars: &[char], start: usize) -> (String, usize) {
    let mut comment = String::from("/*");
    let mut i = start + 2;
    let len = chars.len();
    while i < len {
        comment.push(chars[i]);
        if chars[i] == '*' && i + 1 < len && chars[i + 1] == '/' {
            comment.push(chars[i + 1]);
            i += 2;
            return (comment, i);
        }
        i += 1;
    }
    (comment, i)
}

fn push_brace_breaks(ch: char, brace_depth: i32, next_char: Option<char>, result: &mut String) -> bool {
    if ch == '{' && brace_depth > 0 && next_char.is_some_and(|c| c != '\n') {
        result.push(ch);
        result.push('\n');
        return true;
    }
    if ch == '}' && brace_depth >= 0 && !result.ends_with('\n') {
        result.push('\n');
    }
    false
}

fn process_block_comment(chars: &[char], start: usize, result: &mut String) -> usize {
    emit_block_comment_break(result);
    let (comment, i) = collect_block_comment(chars, start);
    result.push_str(&expand_inline_docblock(&comment));
    if !result.ends_with('\n') {
        result.push('\n');
    }
    if i < chars.len() && chars[i] != '\n' {
        result.push('\n');
    }
    i
}

pub fn normalize_statements(code: &str) -> String {
    let mut result = String::from("\n");
    let chars: Vec<char> = code.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut paren_depth: i32 = 0;
    let mut brace_depth: i32 = 0;
    let mut bracket_depth: i32 = 0;

    while i < len {
        let ch = chars[i];
        if ch == '\'' || ch == '"' {
            i = skip_string_literal(&chars, i, &mut result);
            continue;
        }
        if ch == '/' && i + 1 < len && chars[i + 1] == '*' {
            i = process_block_comment(&chars, i, &mut result);
            continue;
        }
        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth -= 1,
            '{' => brace_depth += 1,
            '}' => brace_depth -= 1,
            '[' => bracket_depth += 1,
            ']' => bracket_depth -= 1,
            _ => {}
        }
        let next = if i + 1 < len { Some(chars[i + 1]) } else { None };
        if push_brace_breaks(ch, brace_depth, next, &mut result) {
            i += 1;
            continue;
        }
        if paren_depth <= 0
            && !result.ends_with('\n')
            && !result.trim_end().is_empty()
            && (matches_keyword_at(&chars, i, "case ") || matches_keyword_at(&chars, i, "default:"))
        {
            result.push('\n');
        }
        result.push(ch);
        let next_is_nl = next.is_some_and(|c| c == '\n');
        if ch == ';' && paren_depth <= 0 && !next_is_nl {
            result.push('\n');
        }
        if ch == ',' && brace_depth > 0 && paren_depth <= 0 && bracket_depth <= 0 && !next_is_nl {
            result.push('\n');
        }
        i += 1;
    }
    result
}

pub fn matches_keyword_at(chars: &[char], pos: usize, keyword: &str) -> bool {
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

impl Formatter {
    pub(crate) fn emit_reindented_line(&self, formatted: &str, pad: &str, depth: &mut i32, result: &mut String) {
        let leading = (count_leading_closers(formatted) as i32).min(1);
        let is_continuation = formatted.starts_with("? ")
            || formatted.starts_with(": ")
            || formatted.starts_with("|| ")
            || formatted.starts_with("&& ")
            || formatted.starts_with(". ")
            || formatted.starts_with("->");
        let extra = i32::from(is_continuation);
        let write_depth = (*depth - leading + extra).max(0) as usize;
        let inner_pad = self.indent.repeat(write_depth);
        let base_pad = format!("{pad}{inner_pad}");
        if let Some((head, tail)) = split_trailing_array_item_close(formatted) {
            result.push_str(&format!("{base_pad}{head}\n"));
            let close_depth = write_depth.saturating_sub(1);
            let close_pad = format!("{pad}{}", self.indent.repeat(close_depth));
            result.push_str(&format!("{close_pad}{tail}\n"));
        } else if let Some(split) = self.try_split_long_line(formatted, &base_pad) {
            result.push_str(&split);
        } else if formatted.starts_with('*') {
            result.push_str(&format!("{pad}{inner_pad} {formatted}\n"));
        } else {
            result.push_str(&format!("{pad}{inner_pad}{formatted}\n"));
        }
        let (openers, closers) = count_brackets(formatted);
        let net = openers as i32 - closers as i32;
        *depth += net.clamp(-1, 1);
        *depth = (*depth).max(0);
    }
}

fn split_trailing_array_item_close(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    let (core, suffix) = if let Some(no_comma) = trimmed.strip_suffix(',') {
        (no_comma.trim_end(), ",")
    } else if let Some(no_semicolon) = trimmed.strip_suffix(';') {
        (no_semicolon.trim_end(), ";")
    } else {
        return None;
    };

    if !core.ends_with(']') {
        return None;
    }

    let (openers, closers) = count_brackets(core);
    if closers != openers + 1 {
        return None;
    }

    let head = core.strip_suffix(']')?.trim_end();
    if head.is_empty() {
        return None;
    }

    let mut first_line = head.to_string();
    if !first_line.ends_with(',') {
        first_line.push(',');
    }

    Some((first_line, format!("]{suffix}")))
}

pub fn join_ternary_lines(code: &str) -> String {
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
        let mut saw_false_branch = false;
        let mut false_branch_depth: i32 = 0;
        i += 1;
        while i < lines.len() {
            let next = lines[i].trim();
            if next.is_empty() {
                break;
            }
            joined.push(' ');
            joined.push_str(next);

            if let Some(false_part) = next.strip_prefix(':').map(str::trim_start) {
                saw_false_branch = true;
                false_branch_depth = 0;
                let (o, c) = count_brackets(false_part);
                false_branch_depth += o as i32 - c as i32;
                if false_branch_depth <= 0 && (false_part.ends_with(',') || false_part.ends_with(';')) {
                    i += 1;
                    break;
                }
            } else if saw_false_branch {
                let (o, c) = count_brackets(next);
                false_branch_depth += o as i32 - c as i32;
                if false_branch_depth <= 0 && (next.ends_with(',') || next.ends_with(';')) {
                    i += 1;
                    break;
                }
            }

            i += 1;
            if next.contains(';') {
                break;
            }
        }
        result.push(joined);
    }

    result.join("\n")
}

pub fn count_unescaped_quotes(line: &str, quote: char) -> usize {
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

pub fn has_unclosed_string(line: &str) -> bool {
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

pub fn detect_open_quote(line: &str) -> Option<char> {
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

pub fn detect_heredoc(line: &str) -> Option<String> {
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

pub fn count_semicolons_outside_parens(code: &str) -> usize {
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

pub fn count_top_level_semicolons(code: &str) -> usize {
    let mut count = 0;
    let mut depth = 0i32;
    let mut in_str = false;
    let mut str_char = '"';
    let chars: Vec<char> = code.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if in_str {
            if c == '\\' {
                i += 1;
            } else if c == str_char {
                in_str = false;
            }
        } else if c == '\'' || c == '"' {
            in_str = true;
            str_char = c;
        } else if matches!(c, '(' | '[' | '{') {
            depth += 1;
        } else if matches!(c, ')' | ']' | '}') {
            depth -= 1;
        } else if c == ';' && depth <= 0 {
            count += 1;
        }
        i += 1;
    }
    count
}

impl Formatter {
    #[allow(clippy::too_many_lines)]
    pub(crate) fn reindent_php_block(&self, code: &str, pad: &str) -> String {
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
        let mut prev_was_declare = false;
        let is_header = is_header_php_block(&code);
        let mut in_string: Option<char> = None;
        let mut heredoc_marker: Option<String> = None;
        let mut pending_docblocks: Vec<String> = Vec::new();
        let mut pending_descriptions: Vec<String> = Vec::new();
        let mut deferred_lines: Vec<String> = Vec::new();

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
                if trimmed == "*/" || trimmed == "**/" {
                    in_docblock = false;
                    let all_var = !docblock_bodies.is_empty() && docblock_bodies.iter().all(|b| b.starts_with("@var "));
                    if all_var {
                        pending_docblocks.append(&mut docblock_bodies);
                    } else if is_header {
                        pending_descriptions.append(&mut docblock_bodies);
                    } else {
                        if !pending_docblocks.is_empty() || !pending_descriptions.is_empty() {
                            self.flush_docblocks(
                                &merge_descriptions_and_vars(&pending_descriptions, &pending_docblocks),
                                pad,
                                &mut depth,
                                &mut result,
                            );
                            pending_docblocks.clear();
                            pending_descriptions.clear();
                        }
                        self.emit_reindented_line("/**", pad, &mut depth, &mut result);
                        for body in &docblock_bodies {
                            self.emit_reindented_line(&format!("* {body}"), pad, &mut depth, &mut result);
                        }
                        self.emit_reindented_line("*/", pad, &mut depth, &mut result);
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
            if has_pending
                && extract_docblock_body(trimmed).is_none()
                && trimmed != "/**"
                && !is_use_import
                && !is_declare
            {
                if !deferred_lines.is_empty() {
                    emit_deferred_lines(self, &deferred_lines, pad, &mut depth, &mut result);
                    result.push('\n');
                    deferred_lines.clear();
                }
                self.flush_docblocks(
                    &merge_descriptions_and_vars(&pending_descriptions, &pending_docblocks),
                    pad,
                    &mut depth,
                    &mut result,
                );
                pending_docblocks.clear();
                pending_descriptions.clear();
                prev_was_doc_close = true;
                prev_blank = false;
            } else if has_pending && (is_use_import || is_declare) {
                deferred_lines.push(trimmed.to_string());
                prev_was_declare = is_declare;
                continue;
            }

            if prev_was_declare && !is_declare && !prev_blank {
                result.push('\n');
            }
            if prev_was_doc_close && !prev_blank {
                result.push('\n');
            }
            prev_blank = false;
            prev_was_doc_close = trimmed == "*/";
            prev_was_declare = is_declare;

            if let Some(body) = extract_docblock_body(trimmed) {
                pending_docblocks.push(body);
                prev_was_declare = false;
                continue;
            }

            if trimmed == "/**" {
                in_docblock = true;
                docblock_bodies.clear();
                continue;
            }

            let formatted = format_php_code(trimmed);
            self.emit_reindented_line(&formatted, pad, &mut depth, &mut result);
            if let Some(marker) = detect_heredoc(trimmed) {
                heredoc_marker = Some(marker);
            } else if has_unclosed_string(trimmed) {
                in_string = detect_open_quote(trimmed);
            }
        }

        if !pending_docblocks.is_empty() || !pending_descriptions.is_empty() {
            if !deferred_lines.is_empty() {
                emit_deferred_lines(self, &deferred_lines, pad, &mut depth, &mut result);
                result.push('\n');
            }
            self.flush_docblocks(
                &merge_descriptions_and_vars(&pending_descriptions, &pending_docblocks),
                pad,
                &mut depth,
                &mut result,
            );
        } else if !deferred_lines.is_empty() {
            emit_deferred_lines(self, &deferred_lines, pad, &mut depth, &mut result);
        }

        let result = result.trim_end_matches('\n').to_string() + "\n";
        sort_use_lines(&result)
    }
}

fn sort_use_lines(code: &str) -> String {
    let lines: Vec<&str> = code.lines().collect();
    let mut result: Vec<String> = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.starts_with("use ") && trimmed.ends_with(';') {
            let mut use_group: Vec<&str> = Vec::new();
            while i < lines.len() && lines[i].trim().starts_with("use ") && lines[i].trim().ends_with(';') {
                use_group.push(lines[i]);
                i += 1;
            }
            use_group.sort_by_key(|a| a.trim().to_lowercase());
            use_group.dedup_by(|a, b| a.trim() == b.trim());

            for line in use_group {
                result.push(line.to_string());
            }
            if i < lines.len() && !lines[i].trim().is_empty() {
                result.push(String::new());
            }
        } else {
            result.push(lines[i].to_string());
            i += 1;
        }
    }

    result.join("\n") + "\n"
}

fn emit_deferred_lines(formatter: &Formatter, deferred: &[String], pad: &str, depth: &mut i32, result: &mut String) {
    let mut declares = Vec::new();
    let mut others = Vec::new();
    for dl in deferred {
        if dl.trim().starts_with("declare(") {
            declares.push(dl.clone());
        } else {
            others.push(dl.clone());
        }
    }
    for dl in &declares {
        formatter.emit_reindented_line(dl, pad, depth, result);
    }
    if !declares.is_empty() && !others.is_empty() {
        result.push('\n');
    }
    for dl in &others {
        formatter.emit_reindented_line(dl, pad, depth, result);
    }
}
