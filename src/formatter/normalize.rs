use super::scan::count_brackets;

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

pub(crate) fn normalize_statements(code: &str) -> String {
    let mut result = String::from("\n");
    let chars: Vec<char> = code.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut paren_depth: i32 = 0;
    let mut brace_depth: i32 = 0;
    let mut bracket_depth: i32 = 0;
    let mut in_case_label = false;

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
        if paren_depth <= 0 && (matches_keyword_at(&chars, i, "case ") || matches_keyword_at(&chars, i, "default:")) {
            if !result.ends_with('\n') && !result.trim_end().is_empty() {
                result.push('\n');
            }
            in_case_label = true;
        }
        if in_case_label && ch == ':' && paren_depth <= 0 && bracket_depth <= 0 {
            if next == Some(':') {
                result.push_str("::");
                i += 2;
                continue;
            }
            result.push_str(":\n");
            in_case_label = false;
            i += 1;
            continue;
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

pub(crate) fn join_ternary_lines(code: &str) -> String {
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

pub(crate) fn join_logical_lines(code: &str) -> String {
    let mut result: Vec<String> = Vec::new();
    for line in code.lines() {
        let trimmed = line.trim_start();
        if is_logical_continuation(trimmed)
            && let Some(last) = result.last_mut()
        {
            last.push(' ');
            last.push_str(trimmed);
            continue;
        }
        result.push(line.to_string());
    }
    result.join("\n")
}

fn is_logical_continuation(trimmed: &str) -> bool {
    trimmed.starts_with("|| ") || trimmed.starts_with("&& ") || trimmed.starts_with(". ")
}
