fn byte_offset(chars: &[char], char_index: usize) -> usize {
    chars[..char_index].iter().map(|c| c.len_utf8()).sum()
}

pub(crate) struct Sig {
    pub(crate) index: usize,
    pub(crate) ch: char,
    pub(crate) depth: i32,
}

pub(crate) struct PhpCursor<'a> {
    chars: &'a [char],
    pos: usize,
    depth: i32,
}

impl<'a> PhpCursor<'a> {
    pub(crate) fn new(chars: &'a [char]) -> Self {
        Self {
            chars,
            pos: 0,
            depth: 0,
        }
    }

    pub(crate) fn from(chars: &'a [char], start: usize) -> Self {
        Self {
            chars,
            pos: start,
            depth: 0,
        }
    }

    pub(crate) fn with_depth(chars: &'a [char], depth: i32) -> Self {
        Self { chars, pos: 0, depth }
    }
}

impl Iterator for PhpCursor<'_> {
    type Item = Sig;

    fn next(&mut self) -> Option<Sig> {
        let len = self.chars.len();
        while self.pos < len {
            let ch = self.chars[self.pos];
            if ch == '\'' || ch == '"' {
                self.pos += 1;
                while self.pos < len && self.chars[self.pos] != ch {
                    if self.chars[self.pos] == '\\' {
                        self.pos += 1;
                    }
                    self.pos += 1;
                }
                if self.pos < len {
                    self.pos += 1;
                }
                continue;
            }
            let index = self.pos;
            self.pos += 1;
            if matches!(ch, '(' | '[' | '{') {
                self.depth += 1;
            } else if matches!(ch, ')' | ']' | '}') {
                self.depth -= 1;
            }
            return Some(Sig {
                index,
                ch,
                depth: self.depth,
            });
        }
        None
    }
}

pub fn find_matching_close(chars: &[char], open_pos: usize) -> Option<usize> {
    PhpCursor::from(chars, open_pos)
        .find(|s| s.depth == 0 && matches!(s.ch, ')' | ']' | '}'))
        .map(|s| s.index)
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
    PhpCursor::new(&chars)
        .find(|s| s.ch == '=' && s.depth == 0 && chars.get(s.index + 1) == Some(&'>'))
        .map(|s| byte_offset(&chars, s.index))
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
    PhpCursor::new(&chars)
        .filter(|s| s.ch == '=' && s.depth == 0)
        .find(|s| is_assignment_equal(&chars, s.index))
        .map(|s| byte_offset(&chars, s.index))
}

fn is_assignment_equal(chars: &[char], pos: usize) -> bool {
    let prev = prev_non_ws(chars, pos);
    let next = next_non_ws(chars, pos);
    let is_non_assignment = next.is_some_and(|c| c == '=' || c == '>')
        || prev.is_some_and(|c| {
            matches!(
                c,
                '=' | '!' | '<' | '>' | '+' | '-' | '*' | '/' | '%' | '.' | '&' | '|' | '^' | '?'
            )
        });
    !is_non_assignment
}

pub fn split_by_commas(code: &str) -> Vec<String> {
    split_segments(code, 0, 0)
}

pub(crate) fn split_by_commas_with_depth(code: &str, start_depth: i32, split_depth: i32) -> Vec<String> {
    split_segments(code, start_depth, split_depth)
}

fn split_segments(code: &str, start_depth: i32, split_depth: i32) -> Vec<String> {
    let chars: Vec<char> = code.chars().collect();
    let mut items = Vec::new();
    let mut start = 0;
    for sig in PhpCursor::with_depth(&chars, start_depth) {
        if sig.ch == ',' && sig.depth == split_depth {
            items.push(chars[start..sig.index].iter().collect::<String>().trim().to_string());
            start = sig.index + 1;
        }
    }
    let last = chars[start..].iter().collect::<String>().trim().to_string();
    if !last.is_empty() {
        items.push(last);
    }
    items
}

pub(crate) fn bracket_balance(code: &str) -> i32 {
    let chars: Vec<char> = code.chars().collect();
    PhpCursor::new(&chars).last().map_or(0, |s| s.depth)
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
    let open = PhpCursor::new(&chars).find(|s| s.ch == '{').map(|s| s.index)?;
    let close = find_matching_close(&chars, open)?;
    Some((open, close))
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

pub(crate) fn count_brackets(s: &str) -> (usize, usize) {
    let chars: Vec<char> = s.chars().collect();
    let mut openers = 0usize;
    let mut closers = 0usize;
    for sig in PhpCursor::new(&chars) {
        if matches!(sig.ch, '(' | '[' | '{') {
            openers += 1;
        } else if matches!(sig.ch, ')' | ']' | '}') {
            closers += 1;
        }
    }
    (openers, closers)
}

pub(crate) fn count_leading_closers(s: &str) -> usize {
    s.chars().take_while(|c| matches!(c, ')' | ']' | '}')).count()
}

pub(crate) fn count_unescaped_quotes(line: &str, quote: char) -> usize {
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

pub(crate) fn has_unclosed_string(line: &str) -> bool {
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

pub(crate) fn detect_open_quote(line: &str) -> Option<char> {
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

pub(crate) fn detect_heredoc(line: &str) -> Option<String> {
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

pub(crate) fn count_semicolons_outside_parens(code: &str) -> usize {
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

pub(crate) fn count_top_level_semicolons(code: &str) -> usize {
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

#[cfg(test)]
#[path = "scan_tests.rs"]
mod tests;
