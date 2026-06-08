use super::Formatter;
use super::indent::visual_len;
use super::php::{split_by_args, split_by_commas};

/// Переводит индекс в `Vec<char>` в байтовый offset исходной строки.
///
/// Сканеры ниже индексируют срез `&[char]`, но результат используется для
/// нарезки исходного `&str`, которая работает по байтам. На многобайтовых
/// символах (кириллица и пр.) символьный индекс не равен байтовому — без
/// перевода это даёт неверный срез или панику «not a char boundary».
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

impl Formatter {
    pub(crate) fn append_ternary_value(&self, result: &mut String, marker: char, value: &str, line_pad: &str) {
        let single_len = line_pad.len() + 2 + value.len();
        if single_len <= self.max_line_length {
            result.push_str(&format!("{line_pad}{marker} {value}\n"));
            return;
        }

        if let Some(split) = self.try_split_long_line(value, line_pad) {
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

    pub(crate) fn try_split_long_line(&self, formatted: &str, base_pad: &str) -> Option<String> {
        if visual_len(base_pad) + visual_len(formatted) <= self.max_line_length {
            return None;
        }

        if let Some(split) = self.split_long_by_commas(formatted, base_pad) {
            return Some(split);
        }

        if let Some((q_pos, c_pos)) = find_ternary_positions(formatted) {
            let condition = formatted[..q_pos].trim_end();
            let true_val = formatted[q_pos + 1..c_pos].trim();
            let false_val = formatted[c_pos + 1..].trim();
            let inner_pad = format!("{base_pad}{}", self.indent);
            let mut result = format!("{base_pad}{condition}\n");
            self.append_ternary_value(&mut result, '?', true_val, &inner_pad);
            self.append_ternary_value(&mut result, ':', false_val, &inner_pad);
            return Some(result);
        }

        if let Some((prefix, args, suffix)) = split_by_args(formatted) {
            return Some(self.build_split(&prefix, &args, &suffix, base_pad));
        }

        if let Some(expanded) = self.expand_assignment_array(formatted, base_pad) {
            return Some(expanded);
        }

        if let Some(expanded) = self.expand_bare_array_with_suffix(formatted, base_pad) {
            return Some(expanded);
        }

        let chars: Vec<char> = formatted.chars().collect();
        if let Some(open_pos) = chars.iter().position(|&c| c == '(')
            && let Some(close_pos) = find_matching_close(&chars, open_pos)
        {
            let inner: String = chars[open_pos + 1..close_pos].iter().collect();
            let inner = inner.trim();

            if let Some(array_inner) = inner.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                let items = split_by_commas(array_inner);
                if items.len() > 1 {
                    let prefix: String = chars[..=open_pos].iter().collect();
                    let suffix: String = chars[close_pos..].iter().collect();
                    let new_prefix = format!("{prefix}[");
                    let new_suffix = format!("]{suffix}");
                    return Some(self.build_split(&new_prefix, &items, &new_suffix, base_pad));
                }
            }
        }

        if let Some(expanded) = self.expand_nested_array(formatted, base_pad) {
            return Some(expanded);
        }

        None
    }
}

fn find_top_level_fat_arrow(code: &str) -> Option<usize> {
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

fn find_top_level_assignment_equal(code: &str) -> Option<usize> {
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

impl Formatter {
    fn format_assignment_array_item(&self, item: &str, pad: &str) -> String {
        let indent = &self.indent;
        let item = item.trim();
        if item.is_empty() {
            return String::new();
        }

        if item.starts_with('[') && item.ends_with(']') {
            if let Some(expanded) = self.expand_assignment_array_literal(item, pad, true) {
                return expanded;
            }
            return format!("{pad}{item},\n");
        }

        if let Some(arrow_pos) = find_top_level_fat_arrow(item) {
            let key = item[..arrow_pos + 2].trim_end();
            let value = item[arrow_pos + 2..].trim_start();
            if value.starts_with('[') && value.ends_with(']') {
                let inner = &value[1..value.len() - 1];
                let sub_items = split_by_commas(inner);
                if sub_items.len() > 1 {
                    let nested_pad = format!("{pad}{indent}");
                    let mut result = format!("{pad}{key} [\n");
                    for sub in &sub_items {
                        result.push_str(&self.format_assignment_array_item(sub, &nested_pad));
                    }
                    result.push_str(&format!("{pad}],\n"));
                    return result;
                }
            }
        }

        if visual_len(pad) + visual_len(item) + 1 > self.max_line_length {
            if let Some(split) = self.try_split_long_line(item, pad) {
                let trimmed = split.trim_end_matches('\n');
                return format!("{trimmed},\n");
            }
        }

        format!("{pad}{item},\n")
    }

    fn expand_assignment_array_literal(&self, array: &str, pad: &str, trailing_comma: bool) -> Option<String> {
        let trimmed = array.trim();
        if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
            return None;
        }

        let inner = &trimmed[1..trimmed.len() - 1];
        let items = split_by_commas(inner);
        if items.is_empty() {
            return None;
        }

        let first = items[0].trim();
        let should_expand = items.len() > 1
            || first.starts_with('[')
            || visual_len(pad) + visual_len(&self.indent) + visual_len(first) + 1 > self.max_line_length;
        if !should_expand {
            return None;
        }

        let nested_pad = format!("{pad}{}", self.indent);
        let mut result = format!("{pad}[\n");
        for item in &items {
            result.push_str(&self.format_assignment_array_item(item, &nested_pad));
        }

        if trailing_comma {
            result.push_str(&format!("{pad}],\n"));
        } else {
            result.push_str(&format!("{pad}]\n"));
        }

        Some(result)
    }

    fn expand_assignment_array(&self, formatted: &str, base_pad: &str) -> Option<String> {
        let eq_pos = find_top_level_assignment_equal(formatted)?;
        let lhs = formatted[..=eq_pos].trim_end();
        let rhs = formatted[eq_pos + 1..].trim_start();

        let rhs_trimmed = rhs.trim();
        let (array_part, suffix) = if let Some(no_semicolon) = rhs_trimmed.strip_suffix(';') {
            (no_semicolon.trim_end(), ";")
        } else {
            (rhs_trimmed, "")
        };

        if !array_part.starts_with('[') || !array_part.ends_with(']') {
            return None;
        }

        let inner = &array_part[1..array_part.len() - 1];
        let items = split_by_commas(inner);
        if items.is_empty() {
            return None;
        }

        let first = items[0].trim();
        let should_expand = items.len() > 1
            || first.starts_with('[')
            || visual_len(base_pad) + visual_len(lhs) + 2 + visual_len(array_part) > self.max_line_length;
        if !should_expand {
            return None;
        }

        let nested_pad = format!("{base_pad}{}", self.indent);
        let mut result = format!("{base_pad}{lhs} [\n");
        for item in &items {
            result.push_str(&self.format_assignment_array_item(item, &nested_pad));
        }
        result.push_str(&format!("{base_pad}]{suffix}\n"));
        Some(result)
    }

    pub(crate) fn build_split(&self, prefix: &str, args: &[String], suffix: &str, pad: &str) -> String {
        let inner_pad = format!("{pad}{}", self.indent);
        let mut result = String::new();
        let prefix_trimmed = prefix.trim();

        if visual_len(pad) + visual_len(prefix_trimmed) > self.max_line_length {
            let mut prefix_parts = split_by_commas(prefix_trimmed);
            if prefix_parts.len() > 1 {
                let last = prefix_parts.pop().unwrap_or_default();
                for part in prefix_parts {
                    let mut line = part.trim().to_string();
                    line.push(',');
                    result.push_str(&format!("{pad}{line}\n"));
                }
                result.push_str(&format!("{pad}{}\n", last.trim()));
            } else {
                result.push_str(&format!("{pad}{prefix_trimmed}\n"));
            }
        } else {
            result.push_str(&format!("{pad}{prefix_trimmed}\n"));
        }

        for arg in args {
            let line_len = inner_pad.len() + arg.len() + 1;
            if line_len > self.max_line_length {
                if let Some(expanded) = self.expand_nested_array(arg, &inner_pad) {
                    result.push_str(&expanded);
                    continue;
                }
                if let Some(expanded) = self.expand_bare_array(arg, &inner_pad) {
                    result.push_str(&expanded);
                    continue;
                }
                if let Some(expanded) = self.expand_inline_closure(arg, &inner_pad) {
                    result.push_str(&expanded);
                    continue;
                }
                if let Some(split) = self.try_split_long_line(arg, &inner_pad) {
                    let trimmed = split.trim_end_matches('\n');
                    result.push_str(trimmed);
                    result.push_str(",\n");
                    continue;
                }
            }
            if let Some(expanded) = self.expand_bare_array(arg, &inner_pad) {
                result.push_str(&expanded);
                continue;
            }
            result.push_str(&format!("{inner_pad}{arg},\n"));
        }
        let suffix_trimmed = suffix.trim();
        let initial_depth = bracket_balance(prefix_trimmed);
        let split_depth = initial_depth - count_leading_closers(suffix_trimmed) as i32;
        if let Some(split) = self.split_long_by_commas_from_depth(suffix_trimmed, pad, initial_depth, split_depth) {
            result.push_str(&split);
            return result;
        }
        if visual_len(pad) + visual_len(suffix_trimmed) > self.max_line_length {
            if let Some(split) = self.try_split_long_line(suffix_trimmed, pad) {
                result.push_str(&split);
                return result;
            }
        }
        result.push_str(&format!("{pad}{suffix_trimmed}\n"));
        result
    }

    fn split_long_by_commas(&self, formatted: &str, pad: &str) -> Option<String> {
        self.split_long_by_commas_from_depth(formatted, pad, 0, 0)
    }

    fn split_long_by_commas_from_depth(
        &self,
        formatted: &str,
        pad: &str,
        start_depth: i32,
        split_depth: i32,
    ) -> Option<String> {
        let parts = split_by_commas_with_depth(formatted, start_depth, split_depth);
        if parts.len() <= 1 {
            return None;
        }

        let mut result = String::new();
        for (idx, part) in parts.iter().enumerate() {
            let mut line = part.trim().to_string();
            if idx < parts.len() - 1 {
                line.push(',');
            }

            if visual_len(pad) + visual_len(&line) > self.max_line_length {
                if let Some(expanded) = self.expand_bare_array_with_suffix(&line, pad) {
                    result.push_str(&expanded);
                    continue;
                }
                if let Some(expanded) = self.expand_nested_array(&line, pad) {
                    result.push_str(&expanded);
                    continue;
                }
                if let Some(expanded) = self.expand_bare_array(&line, pad) {
                    result.push_str(&expanded);
                    continue;
                }
                if let Some(expanded) = self.expand_inline_closure(&line, pad) {
                    result.push_str(&expanded);
                    continue;
                }
                if let Some(split) = self.try_split_long_line(&line, pad) {
                    result.push_str(&split);
                    continue;
                }
            }

            result.push_str(&format!("{pad}{line}\n"));
        }

        Some(result)
    }

    fn expand_bare_array_with_suffix(&self, line: &str, pad: &str) -> Option<String> {
        let trimmed = line.trim();
        let (array_part, suffix) = if let Some(s) = trimmed.strip_suffix(',') {
            (s.trim_end(), ",")
        } else if let Some(s) = trimmed.strip_suffix(';') {
            (s.trim_end(), ";")
        } else {
            (trimmed, "")
        };

        let mut expanded = self.expand_bare_array(array_part, pad)?;

        match suffix {
            "," => Some(expanded),
            ";" => {
                if expanded.ends_with(",\n") {
                    expanded.truncate(expanded.len() - 2);
                    expanded.push_str(";\n");
                }
                Some(expanded)
            }
            "" => {
                if expanded.ends_with(",\n") {
                    expanded.truncate(expanded.len() - 2);
                    expanded.push('\n');
                }
                Some(expanded)
            }
            _ => None,
        }
    }
}

fn split_by_commas_with_depth(code: &str, mut depth: i32, split_depth: i32) -> Vec<String> {
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

fn bracket_balance(code: &str) -> i32 {
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

fn count_leading_closers(code: &str) -> usize {
    code.chars().take_while(|c| matches!(c, ')' | ']' | '}')).count()
}

impl Formatter {
    pub(crate) fn expand_bare_array(&self, arg: &str, pad: &str) -> Option<String> {
        let indent = &self.indent;
        let trimmed = arg.trim();
        if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
            return None;
        }
        let inner = &trimmed[1..trimmed.len() - 1];
        let items = split_by_commas(inner);
        if items.len() <= 1 {
            if items.len() == 1 {
                let item = &items[0];
                let nested_pad = format!("{pad}{indent}");
                let item_line_len = nested_pad.len() + item.len() + 1;
                if item_line_len > self.max_line_length {
                    let mut result = format!("{pad}[\n");
                    if let Some(split) = self.try_split_long_line(item, &nested_pad) {
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
        let nested_pad = format!("{pad}{indent}");
        let mut result = format!("{pad}[\n");
        for item in &items {
            let item_line_len = nested_pad.len() + item.len() + 1;
            if item_line_len > self.max_line_length {
                if let Some(expanded) = self.expand_nested_array(item, &nested_pad) {
                    result.push_str(&expanded);
                    continue;
                }
            }
            if item.starts_with('[') && item.ends_with(']') {
                let sub_inner = &item[1..item.len() - 1];
                let sub_items = split_by_commas(sub_inner);
                if sub_items.len() > 1 {
                    let deeper_pad = format!("{nested_pad}{indent}");
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

    pub(crate) fn expand_bare_sub_array(&self, item: &str, pad: &str) -> Option<String> {
        if !item.starts_with('[') || !item.ends_with(']') {
            return None;
        }
        let sub_inner = &item[1..item.len() - 1];
        let sub_items = split_by_commas(sub_inner);
        if sub_items.len() <= 1 {
            return None;
        }
        let deeper_pad = format!("{pad}{}", self.indent);
        let mut result = format!("{pad}[\n");
        for sub in &sub_items {
            let sub_line_len = deeper_pad.len() + sub.len() + 1;
            if sub_line_len > self.max_line_length {
                if let Some(expanded) = self.expand_nested_array(sub, &deeper_pad) {
                    result.push_str(&expanded);
                    continue;
                }
                if let Some(expanded) = self.expand_inline_closure(sub, &deeper_pad) {
                    result.push_str(&expanded);
                    continue;
                }
                if let Some(split) = self.try_split_long_line(sub, &deeper_pad) {
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

impl Formatter {
    pub(crate) fn expand_brace_block(&self, stmt: &str, pad: &str) -> Option<String> {
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
        let inner_pad = format!("{pad}{}", self.indent);
        let body_stmts = normalize_closure_body(body);
        if body_stmts.is_empty() {
            return None;
        }
        let mut result = format!("{pad}{header} {{\n");
        for s in &body_stmts {
            let line_len = inner_pad.len() + s.len();
            if line_len > self.max_line_length {
                if let Some(split) = self.try_split_long_line(s, &inner_pad) {
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

    pub(crate) fn expand_inline_closure(&self, arg: &str, pad: &str) -> Option<String> {
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
        let body_pad = format!("{pad}{}", self.indent);
        let mut result = format!("{pad}{header} {{\n");
        for stmt in &stmts {
            if let Some(expanded) = self.expand_brace_block(stmt, &body_pad) {
                result.push_str(&expanded);
                continue;
            }
            let line_len = body_pad.len() + stmt.len();
            if line_len > self.max_line_length {
                if let Some(split) = self.try_split_long_line(stmt, &body_pad) {
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

impl Formatter {
    pub(crate) fn expand_nested_array(&self, arg: &str, pad: &str) -> Option<String> {
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
        let nested_pad = format!("{pad}{}", self.indent);
        let mut result = format!("{pad}{key} [\n");
        for item in &items {
            if visual_len(&nested_pad) + visual_len(item) + 1 > self.max_line_length {
                if let Some(expanded) = self.expand_nested_array(item, &nested_pad) {
                    result.push_str(&expanded);
                    continue;
                }
                if let Some(bare) = self.expand_bare_sub_array(item, &nested_pad) {
                    result.push_str(&bare);
                    continue;
                }
                if let Some(expanded) = self.expand_inline_closure(item, &nested_pad) {
                    result.push_str(&expanded);
                    continue;
                }
                if let Some(split) = self.try_split_long_line(item, &nested_pad) {
                    let trimmed = split.trim_end_matches('\n');
                    result.push_str(trimmed);
                    result.push_str(",\n");
                    continue;
                }
            }
            if let Some(bare) = self.expand_bare_sub_array(item, &nested_pad) {
                result.push_str(&bare);
                continue;
            }
            result.push_str(&format!("{nested_pad}{item},\n"));
        }
        result.push_str(&format!("{pad}],\n"));
        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn fat_arrow_returns_byte_offset_with_cyrillic() {
        let code = "'абв' => 'значение'";
        let pos = find_top_level_fat_arrow(code).unwrap();
        assert_eq!(&code[pos..pos + 2], "=>");
    }

    #[test]
    fn assignment_equal_returns_byte_offset_with_cyrillic() {
        let code = "$абв = ['x' => 'y']";
        let pos = find_top_level_assignment_equal(code).unwrap();
        assert_eq!(&code[pos..=pos], "=");
        let lhs = code[..=pos].trim_end();
        assert_eq!(lhs, "$абв =");
    }

    #[test]
    fn byte_offset_matches_ascii_char_index() {
        let chars: Vec<char> = "abc=>d".chars().collect();
        assert_eq!(byte_offset(&chars, 3), 3);
    }
}
