use super::indent::{INDENT, MAX_LINE_LENGTH, contains_outside_strings};
use super::php::{format_php_code, join_php_lines, split_by_args, split_by_chain, split_by_commas, split_by_concat};
use super::split::{
    expand_bare_array, expand_bare_sub_array, expand_nested_array, find_ternary_positions, try_split_long_line,
};

pub fn is_single_echo_block(code: &str) -> bool {
    let trimmed = code.trim();
    trimmed.starts_with("echo ") && !trimmed.contains('\n') && trimmed.matches(';').count() <= 1
}

pub fn is_echo_block_opener(code: &str) -> bool {
    let trimmed = code.trim().to_lowercase();
    trimmed.contains("begintag(") || trimmed.contains("::begin(")
}

pub fn is_echo_block_closer(code: &str) -> bool {
    let trimmed = code.trim().to_lowercase();
    trimmed.contains("endtag(") || trimmed.contains("::end(")
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

pub fn format_echo(code: &str, pad: &str) -> String {
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

pub fn contains_break(code: &str) -> bool {
    let lower = code.trim().to_lowercase();
    contains_outside_strings(&lower, "break;")
}
