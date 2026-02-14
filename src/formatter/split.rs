use super::indent::{INDENT, MAX_LINE_LENGTH};
use super::php::{split_by_args, split_by_commas};

pub fn find_matching_close(chars: &[char], open_pos: usize) -> Option<usize> {
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

pub fn find_ternary_positions(code: &str) -> Option<(usize, usize)> {
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

pub fn append_ternary_value(result: &mut String, marker: char, value: &str, line_pad: &str) {
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

pub fn try_split_long_line(formatted: &str, base_pad: &str) -> Option<String> {
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

pub fn build_split(prefix: &str, args: &[String], suffix: &str, pad: &str) -> String {
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

pub fn expand_bare_array(arg: &str, pad: &str) -> Option<String> {
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

pub fn expand_bare_sub_array(item: &str, pad: &str) -> Option<String> {
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

pub fn find_closure_body(code: &str) -> Option<(usize, usize)> {
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

pub fn normalize_closure_body(body: &str) -> Vec<String> {
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

pub fn find_brace_block(code: &str) -> Option<(usize, usize)> {
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

pub fn expand_brace_block(stmt: &str, pad: &str) -> Option<String> {
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

pub fn expand_inline_closure(arg: &str, pad: &str) -> Option<String> {
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

pub fn find_array_arrow(arg: &str) -> Option<(usize, usize)> {
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

pub fn expand_nested_array(arg: &str, pad: &str) -> Option<String> {
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
