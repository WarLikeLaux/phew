use super::php::split_by_commas;

fn byte_offset(chars: &[char], char_index: usize) -> usize {
    chars[..char_index].iter().map(|c| c.len_utf8()).sum()
}

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

pub(crate) fn array_is_list(inner: &str) -> bool {
    let items = split_by_commas(inner);
    !items.is_empty()
        && items
            .iter()
            .filter(|item| !item.trim().is_empty())
            .all(|item| find_top_level_fat_arrow(item).is_none())
}

pub(crate) fn find_top_level_fat_arrow(code: &str) -> Option<usize> {
    let chars: Vec<char> = code.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut depth = 0i32;

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
            i += 1;
            continue;
        }

        if matches!(ch, '(' | '[' | '{') {
            depth += 1;
        } else if matches!(ch, ')' | ']' | '}') {
            depth -= 1;
        } else if ch == '=' && i + 1 < len && chars[i + 1] == '>' && depth == 0 {
            return Some(byte_offset(&chars, i));
        }

        i += 1;
    }

    None
}

fn prev_non_ws(chars: &[char], pos: usize) -> Option<char> {
    if pos == 0 {
        return None;
    }
    let mut i = pos;
    while i > 0 {
        i -= 1;
        if !chars[i].is_whitespace() {
            return Some(chars[i]);
        }
    }
    None
}

fn next_non_ws(chars: &[char], pos: usize) -> Option<char> {
    let mut i = pos + 1;
    while i < chars.len() {
        if !chars[i].is_whitespace() {
            return Some(chars[i]);
        }
        i += 1;
    }
    None
}

pub(crate) fn find_top_level_assignment_equal(code: &str) -> Option<usize> {
    let chars: Vec<char> = code.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut depth = 0i32;

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
            i += 1;
            continue;
        }

        if matches!(ch, '(' | '[' | '{') {
            depth += 1;
            i += 1;
            continue;
        }
        if matches!(ch, ')' | ']' | '}') {
            depth -= 1;
            i += 1;
            continue;
        }

        if ch == '=' && depth == 0 {
            let prev = prev_non_ws(&chars, i);
            let next = next_non_ws(&chars, i);
            let is_non_assignment = next.is_some_and(|c| c == '=' || c == '>')
                || prev.is_some_and(|c| {
                    matches!(
                        c,
                        '=' | '!' | '<' | '>' | '+' | '-' | '*' | '/' | '%' | '.' | '&' | '|' | '^' | '?'
                    )
                });
            if !is_non_assignment {
                return Some(byte_offset(&chars, i));
            }
        }

        i += 1;
    }

    None
}

pub(crate) fn split_by_commas_with_depth(code: &str, mut depth: i32, split_depth: i32) -> Vec<String> {
    let chars: Vec<char> = code.chars().collect();
    let len = chars.len();
    let mut items = Vec::new();
    let mut current = String::new();
    let mut i = 0;

    while i < len {
        let ch = chars[i];
        if ch == '\'' || ch == '"' {
            current.push(ch);
            i += 1;
            while i < len && chars[i] != ch {
                if chars[i] == '\\' {
                    current.push(chars[i]);
                    i += 1;
                }
                if i < len {
                    current.push(chars[i]);
                    i += 1;
                }
            }
            if i < len {
                current.push(chars[i]);
                i += 1;
            }
            continue;
        }

        if matches!(ch, '(' | '[' | '{') {
            depth += 1;
        } else if matches!(ch, ')' | ']' | '}') {
            depth -= 1;
        } else if ch == ',' && depth == split_depth {
            items.push(current.trim().to_string());
            current = String::new();
            i += 1;
            continue;
        }

        current.push(ch);
        i += 1;
    }

    if !current.trim().is_empty() {
        items.push(current.trim().to_string());
    }

    items
}

pub(crate) fn bracket_balance(code: &str) -> i32 {
    let chars: Vec<char> = code.chars().collect();
    let len = chars.len();
    let mut depth = 0i32;
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
            i += 1;
            continue;
        }

        if matches!(ch, '(' | '[' | '{') {
            depth += 1;
        } else if matches!(ch, ')' | ']' | '}') {
            depth -= 1;
        }
        i += 1;
    }

    depth
}

pub fn has_expandable_closure(code: &str) -> bool {
    find_closure_body(code).is_some_and(|(open, close)| {
        code.chars()
            .skip(open + 1)
            .take(close - open - 1)
            .any(|c| !c.is_whitespace())
    })
}

pub fn find_closure_body(code: &str) -> Option<(usize, usize)> {
    let chars: Vec<char> = code.chars().collect();
    let len = chars.len();
    let mut i = 0;
    while i < len {
        if chars[i] == '\'' || chars[i] == '"' {
            let quote = chars[i];
            i += 1;
            while i < len && chars[i] != quote {
                if chars[i] == '\\' {
                    i += 1;
                }
                i += 1;
            }
            i += 1;
            continue;
        }
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

#[cfg(test)]
#[path = "scan_tests.rs"]
mod tests;
