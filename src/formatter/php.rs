use super::scan::{find_matching_close, split_by_commas};

const PHP_KEYWORDS: &[&str] = &[
    "if", "elseif", "else", "foreach", "for", "while", "switch", "catch", "match", "fn", "function",
];

pub fn format_php_code(code: &str) -> String {
    let mut result = String::with_capacity(code.len());
    let chars: Vec<char> = code.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let preserve_declare_equal = code.trim_start().starts_with("declare(");

    while i < len {
        let ch = chars[i];

        if ch == '\'' || ch == '"' {
            i = skip_string_literal(&chars, i, &mut result);
            continue;
        }

        if ch == '<' && chars.get(i + 1) == Some(&'=') && chars.get(i + 2) == Some(&'>') {
            if !result.ends_with(' ') {
                result.push(' ');
            }
            result.push_str("<=>");
            let next = i + 3;
            if next < len && chars[next] != ' ' {
                result.push(' ');
            }
            i = next;
            continue;
        }

        if ch == '=' && i + 1 < len && chars[i + 1] == '>' && !result.ends_with('<') {
            i = format_fat_arrow(&chars, i, &mut result);
            continue;
        }

        if ch == '=' && !preserve_declare_equal {
            i = format_assignment_equal(&chars, i, &mut result);
            continue;
        }

        if ch == ',' {
            i = format_comma(&chars, i, &mut result);
            continue;
        }

        if ch.is_alphabetic() {
            i = format_keyword(&chars, i, &mut result);
            continue;
        }

        if matches!(ch, ')' | ']') {
            while result.ends_with(' ') || result.ends_with(',') {
                result.pop();
            }
            result.push(ch);
            i += 1;
            continue;
        }

        if matches!(ch, '(' | '[') {
            result.push(ch);
            i += 1;
            while i < len && chars[i] == ' ' {
                i += 1;
            }
            continue;
        }

        if ch == '{' && result.ends_with(')') {
            result.push(' ');
            result.push('{');
            i += 1;
            continue;
        }

        result.push(ch);
        i += 1;
    }

    result
}

pub fn join_php_lines(code: &str) -> String {
    code.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .replace(" ->", "->")
}

fn has_method_call_after(chars: &[char], start: usize) -> bool {
    let len = chars.len();
    let mut i = start;
    while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
        i += 1;
    }
    i > start && i < len && chars[i] == '('
}

pub fn split_by_chain(code: &str) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = code.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut depth = 0i32;

    while i < len {
        if chars[i] == '\'' || chars[i] == '"' {
            let quote = chars[i];
            current.push(quote);
            i += 1;
            while i < len && chars[i] != quote {
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

        if matches!(chars[i], '(' | '[' | '{') {
            depth += 1;
        } else if matches!(chars[i], ')' | ']' | '}') {
            depth -= 1;
        }

        if depth == 0 && chars[i] == '-' && i + 1 < len && chars[i + 1] == '>' {
            let prev = current.trim_end().chars().last().unwrap_or(' ');
            let next_has_call = has_method_call_after(&chars, i + 2);
            if prev == ')' || next_has_call {
                parts.push(current.trim_end().to_string());
                current = String::from("->");
                i += 2;
                continue;
            }
        }

        current.push(chars[i]);
        i += 1;
    }

    if !current.is_empty() {
        parts.push(current.trim_end().to_string());
    }

    parts
}

pub fn split_by_concat(code: &str) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = code.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut depth = 0i32;

    while i < len {
        let ch = chars[i];

        if ch == '\'' || ch == '"' {
            let quote = ch;
            current.push(ch);
            i += 1;
            while i < len && chars[i] != quote {
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

        if matches!(ch, '(' | '[' | '{') {
            depth += 1;
        } else if matches!(ch, ')' | ']' | '}') {
            depth -= 1;
        }

        if depth == 0 && ch == '.' {
            let prev = current.trim_end().chars().last();
            let mut j = i + 1;
            while j < len && chars[j].is_whitespace() {
                j += 1;
            }
            let next = if j < len { Some(chars[j]) } else { None };

            let is_decimal_point = prev.is_some_and(|c| c.is_ascii_digit()) && next.is_some_and(|c| c.is_ascii_digit());

            if prev.is_some() && next.is_some() && !is_decimal_point {
                parts.push(current.trim_end().to_string());
                current.clear();
                i += 1;
                while i < len && chars[i].is_whitespace() {
                    i += 1;
                }
                continue;
            }
        }

        current.push(ch);
        i += 1;
    }

    if !current.trim().is_empty() {
        parts.push(current.trim_end().to_string());
    }

    parts
}

pub fn split_by_args(code: &str) -> Option<(String, Vec<String>, String)> {
    let chars: Vec<char> = code.chars().collect();
    let open_pos = find_call_open_paren(&chars)?;
    let close_pos = find_matching_close(&chars, open_pos)?;

    let prefix: String = chars[..=open_pos].iter().collect();
    let inner: String = chars[open_pos + 1..close_pos].iter().collect();
    let suffix: String = chars[close_pos..].iter().collect();

    let args = split_by_commas(&inner);
    if args.len() <= 1 {
        return None;
    }
    Some((prefix, args, suffix))
}

fn find_call_open_paren(chars: &[char]) -> Option<usize> {
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
        } else if ch == '(' {
            let prefix: String = chars[..i].iter().collect();
            let trimmed = prefix.trim_end();
            if trimmed.ends_with("fn") || trimmed.ends_with("function") {
                i = skip_paren_group(chars, i + 1);
                continue;
            }
            return Some(i);
        }
        i += 1;
    }
    None
}

fn skip_paren_group(chars: &[char], start: usize) -> usize {
    let len = chars.len();
    let mut depth = 1i32;
    let mut i = start;
    while i < len && depth > 0 {
        let ch = chars[i];
        if ch == '\'' || ch == '"' {
            i += 1;
            while i < len && chars[i] != ch {
                if chars[i] == '\\' {
                    i += 1;
                }
                i += 1;
            }
        } else if ch == '(' {
            depth += 1;
        } else if ch == ')' {
            depth -= 1;
        }
        i += 1;
    }
    i
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
            if i < len {
                result.push(chars[i]);
                i += 1;
            }
            continue;
        }
        result.push(chars[i]);
        i += 1;
    }

    if i < len {
        result.push(chars[i]);
        i += 1;
    }

    i
}

fn format_fat_arrow(chars: &[char], start: usize, result: &mut String) -> usize {
    if !result.ends_with(' ') {
        result.push(' ');
    }
    result.push_str("=>");
    let i = start + 2;
    if i < chars.len() && chars[i] != ' ' {
        result.push(' ');
    }
    i
}

fn format_comma(chars: &[char], start: usize, result: &mut String) -> usize {
    result.push(',');
    let i = start + 1;
    if i < chars.len() && chars[i] != ' ' && chars[i] != '\n' {
        result.push(' ');
    }
    i
}

fn format_assignment_equal(chars: &[char], start: usize, result: &mut String) -> usize {
    let len = chars.len();
    let prev = if start > 0 { Some(chars[start - 1]) } else { None };
    let next = if start + 1 < len { Some(chars[start + 1]) } else { None };

    let is_non_assignment = next.is_some_and(|c| c == '=' || c == '>')
        || prev.is_some_and(|c| {
            matches!(
                c,
                '=' | '!' | '<' | '>' | '+' | '-' | '*' | '/' | '%' | '.' | '&' | '|' | '^' | '?'
            )
        });

    if is_non_assignment {
        result.push('=');
        return start + 1;
    }

    if !result.ends_with(' ') && !result.ends_with('\n') && !result.ends_with('\t') {
        result.push(' ');
    }
    result.push('=');

    let mut i = start + 1;
    while i < len && chars[i] == ' ' {
        i += 1;
    }
    if i < len && chars[i] != ' ' && chars[i] != '\n' && chars[i] != '\t' && chars[i] != '\r' {
        result.push(' ');
    }

    i
}

fn format_keyword(chars: &[char], start: usize, result: &mut String) -> usize {
    let len = chars.len();
    let mut i = start;

    while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
        i += 1;
    }
    let word: String = chars[start..i].iter().collect();

    if PHP_KEYWORDS.contains(&word.as_str()) && i < len && chars[i] == '(' {
        result.push_str(&word);
        result.push(' ');
    } else {
        result.push_str(&word);
    }

    i
}

#[cfg(test)]
#[path = "php_tests.rs"]
mod tests;
