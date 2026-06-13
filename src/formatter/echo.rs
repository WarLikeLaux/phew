use super::Formatter;
use super::indent::visual_len;
use super::php::{format_php_code, join_php_lines, split_by_args, split_by_chain, split_by_concat};
use super::scan::{
    contains_heredoc, contains_outside_strings, count_top_level_semicolons, find_array_arrow, find_matching_close,
    find_ternary_positions, has_expandable_closure, split_by_commas,
};

const WIDGET_MARKER: &str = "::widget(";

pub(crate) fn contains_line_comment(code: &str) -> bool {
    let chars: Vec<char> = code.chars().collect();
    let len = chars.len();
    let mut i = 0;
    while i < len {
        let ch = chars[i];
        if ch == '\'' || ch == '"' {
            i = skip_echo_string(&chars, i);
            continue;
        }
        if ch == '/' && chars.get(i + 1) == Some(&'*') {
            i = skip_echo_block_comment(&chars, i);
            continue;
        }
        if (ch == '/' && chars.get(i + 1) == Some(&'/')) || (ch == '#' && chars.get(i + 1) != Some(&'[')) {
            return true;
        }
        i += 1;
    }
    false
}

fn skip_echo_block_comment(chars: &[char], start: usize) -> usize {
    let mut i = start + 2;
    while i + 1 < chars.len() {
        if chars[i] == '*' && chars[i + 1] == '/' {
            return i + 2;
        }
        i += 1;
    }
    chars.len()
}

fn skip_echo_string(chars: &[char], start: usize) -> usize {
    let quote = chars[start];
    let mut i = start + 1;
    while i < chars.len() {
        if chars[i] == '\\' {
            i += 2;
            continue;
        }
        if chars[i] == quote {
            return i + 1;
        }
        i += 1;
    }
    i
}

pub fn is_single_echo_block(code: &str) -> bool {
    let trimmed = code.trim();
    trimmed.starts_with("echo ") && !trimmed.contains('\n') && trimmed.matches(';').count() <= 1
}

fn widget_config_is_structural(body: &str) -> bool {
    if has_expandable_closure(body) {
        return true;
    }
    split_by_commas(body).iter().any(|item| {
        find_array_arrow(item)
            .map(|(skip, arrow)| item[skip + arrow + 2..].trim_start().starts_with('['))
            .unwrap_or(false)
    })
}

fn is_structural_widget(code: &str) -> bool {
    let chars: Vec<char> = code.chars().collect();
    let marker: Vec<char> = WIDGET_MARKER.chars().collect();
    let Some(start) = chars.windows(marker.len()).position(|w| w == marker.as_slice()) else {
        return false;
    };
    let open = start + marker.len() - 1;
    let Some(close) = find_matching_close(&chars, open) else {
        return false;
    };
    let inner: String = chars[open + 1..close].iter().collect();
    let inner = inner.trim();
    let Some(body) = inner.strip_prefix('[').and_then(|s| s.strip_suffix(']')) else {
        return false;
    };
    widget_config_is_structural(body)
}

fn strip_echo_semicolon(code: &str) -> &str {
    let trimmed = code.trim();
    if count_top_level_semicolons(trimmed) == 1 {
        return trimmed.strip_suffix(';').map(str::trim_end).unwrap_or(trimmed);
    }
    trimmed
}

pub fn is_echo_block_opener(code: &str) -> bool {
    let trimmed = code.trim().to_lowercase();
    trimmed.contains("begintag(") || trimmed.contains("::begin(")
}

pub fn is_echo_block_closer(code: &str) -> bool {
    let trimmed = code.trim().to_lowercase();
    trimmed.contains("endtag(") || trimmed.contains("::end(")
}

impl Formatter {
    fn format_echo_chain(&self, parts: &[String], pad: &str) -> String {
        let chain_pad = format!("{pad}{}", self.indent);
        let mut result = format!("{pad}<?= {}{}", parts[0], parts[1]);
        for part in &parts[2..] {
            let part_line_len = visual_len(&chain_pad) + visual_len(part);
            if part_line_len > self.max_line_length {
                if let Some(split) = self.try_split_long_line(part, &chain_pad) {
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

    fn format_echo_concat(&self, parts: &[String], pad: &str) -> String {
        let concat_pad = format!("{pad}{}", self.indent);
        let mut result = format!("{pad}<?= {}", parts[0]);
        for part in &parts[1..] {
            result.push_str(&format!("\n{concat_pad}. {part}"));
        }
        result.push_str(" ?>\n");
        result
    }

    fn split_ternary(&self, code: &str, pad: &str) -> Option<String> {
        let (q_pos, c_pos) = find_ternary_positions(code)?;

        let condition = code[..q_pos].trim_end();
        let true_val = code[q_pos + 1..c_pos].trim();
        let false_val = code[c_pos + 1..].trim();

        let inner_pad = format!("{pad}{}", self.indent);
        Some(format!(
            "{pad}<?= {condition}\n{inner_pad}? {true_val}\n{inner_pad}: {false_val} ?>\n"
        ))
    }

    pub fn format_echo(&self, code: &str, pad: &str) -> String {
        if contains_heredoc(code) {
            return format_heredoc_echo(code, pad);
        }
        if contains_line_comment(code) {
            return format_multiline_echo(code, pad);
        }
        let joined = join_php_lines(code);
        let joined = strip_echo_semicolon(&joined);
        let formatted = format_php_code(joined);
        let single = format!("{pad}<?= {formatted} ?>");

        let fits = visual_len(&single) <= self.max_line_length && !has_expandable_closure(&formatted);
        if fits && !is_structural_widget(&formatted) {
            return format!("{single}\n");
        }

        let parts = split_by_chain(&formatted);
        if parts.len() > 2 {
            return self.format_echo_chain(&parts, pad);
        }

        if let Some(result) = self.split_ternary(&formatted, pad) {
            return result;
        }

        let concat_parts = split_by_concat(&formatted);
        if concat_parts.len() > 1 {
            return self.format_echo_concat(&concat_parts, pad);
        }

        if let Some((prefix, args, suffix)) = split_by_args(&formatted) {
            return self.format_echo_call(&prefix, &args, &suffix, pad);
        }

        if let Some(split) = self.split_long_line(&formatted, pad) {
            let trimmed = split.trim_start().trim_end_matches('\n');
            return format!("{pad}<?= {trimmed} ?>\n");
        }

        format!("{single}\n")
    }

    fn format_echo_call(&self, prefix: &str, args: &[String], suffix: &str, pad: &str) -> String {
        let inner_pad = format!("{pad}{}", self.indent);
        let mut result = format!("{pad}<?= {prefix}\n");
        for arg in args {
            result.push_str(&self.format_echo_arg(arg, &inner_pad));
        }
        result.push_str(&format!("{pad}{suffix} ?>\n"));
        result
    }

    fn format_echo_arg(&self, arg: &str, inner_pad: &str) -> String {
        let line_len = visual_len(inner_pad) + visual_len(arg) + 1;
        if line_len > self.max_line_length {
            if let Some(expanded) = self.expand_nested_array(arg, inner_pad) {
                return expanded;
            }
            if let Some(expanded) = self.expand_bare_array(arg, inner_pad) {
                return expanded;
            }
            if let Some(split) = self.try_split_long_line(arg, inner_pad) {
                return format!("{},\n", split.trim_end_matches('\n'));
            }
        }
        if let Some(expanded) = self.expand_bare_array(arg, inner_pad) {
            return expanded;
        }
        if let Some(expanded) = self.expand_closure_element(arg, inner_pad) {
            return expanded;
        }
        format!("{inner_pad}{arg},\n")
    }
}

fn format_multiline_echo(code: &str, pad: &str) -> String {
    let lines: Vec<&str> = code.lines().filter(|line| !line.trim().is_empty()).collect();
    match lines.as_slice() {
        [] => format!("{pad}<?= ?>\n"),
        [single] => format!("{pad}<?= {} ?>\n", single.trim()),
        [first, rest @ ..] => {
            let common = common_leading_whitespace(rest);
            let mut result = format!("{pad}<?= {}\n", first.trim_start());
            for line in &rest[..rest.len().saturating_sub(1)] {
                result.push_str(pad);
                result.push_str(strip_leading_width(line, common).trim_end());
                result.push('\n');
            }
            let last = rest.last().copied().unwrap_or("");
            result.push_str(pad);
            result.push_str(strip_echo_semicolon(strip_leading_width(last, common)));
            result.push_str(" ?>\n");
            result
        }
    }
}

fn common_leading_whitespace(lines: &[&str]) -> usize {
    lines
        .iter()
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0)
}

fn strip_leading_width(line: &str, width: usize) -> &str {
    line.get(width..).unwrap_or_else(|| line.trim_start())
}

fn format_heredoc_echo(code: &str, pad: &str) -> String {
    let mut lines = code.lines();
    let first = lines.next().unwrap_or("").trim();
    let mut result = format!("{pad}<?= {first}");
    for line in lines {
        result.push('\n');
        result.push_str(line.trim_end());
    }
    result.push_str(" ?>\n");
    result
}

pub fn contains_break(code: &str) -> bool {
    let lower = code.trim().to_lowercase();
    contains_outside_strings(&lower, "break;")
}
