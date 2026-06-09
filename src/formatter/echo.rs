use super::Formatter;
use super::indent::{contains_outside_strings, visual_len};
use super::php::{format_php_code, join_php_lines, split_by_args, split_by_chain, split_by_commas, split_by_concat};
use super::split::{find_array_arrow, find_matching_close, find_ternary_positions, has_expandable_closure};

const WIDGET_MARKER: &str = "::widget(";

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
            let part_line_len = chain_pad.len() + part.len();
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
        let joined = join_php_lines(code);
        let formatted = format_php_code(&joined);
        let combined = format!("{formatted} ?>");
        let single = format!("{pad}<?= {combined}");

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
            let mut result = format!("{pad}<?= {prefix}\n");
            let inner_pad = format!("{pad}{}", self.indent);
            for arg in &args {
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
                if let Some(expanded) = self.expand_closure_element(arg, &inner_pad) {
                    result.push_str(&expanded);
                    continue;
                }
                result.push_str(&format!("{inner_pad}{arg},\n"));
            }
            result.push_str(&format!("{pad}{suffix} ?>\n"));
            return result;
        }

        if let Some(split) = self.split_long_line(&formatted, pad) {
            let trimmed = split.trim_start().trim_end_matches('\n');
            return format!("{pad}<?= {trimmed} ?>\n");
        }

        format!("{single}\n")
    }
}

pub fn contains_break(code: &str) -> bool {
    let lower = code.trim().to_lowercase();
    contains_outside_strings(&lower, "break;")
}
