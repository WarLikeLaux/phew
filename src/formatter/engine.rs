use super::php::{format_php_code, join_php_lines, split_by_args, split_by_chain, split_by_commas};
use crate::parser::ast::Node;
use crate::parser::lexer::Attribute;

const INDENT: &str = "    ";
const MAX_LINE_LENGTH: usize = 120;

const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source", "track", "wbr",
];

const RAW_TEXT_ELEMENTS: &[&str] = &["script", "style", "textarea"];

fn contains_outside_strings(code: &str, needle: &str) -> bool {
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

fn has_switch_case(code: &str) -> bool {
    let lower = code.to_lowercase();
    (lower.contains("switch") || contains_outside_strings(&lower, "break"))
        && (lower.contains("case ") || lower.contains("default:"))
}

fn is_php_block_opener(code: &str) -> bool {
    let trimmed = code.trim();
    trimmed.ends_with(':') || trimmed.ends_with('{') || trimmed.contains("::begin(")
}

fn is_php_block_closer(code: &str) -> bool {
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

fn is_switch_case_peer(code: &str) -> bool {
    let lower = code.trim().to_lowercase();
    lower.starts_with("case ") || lower.starts_with("default:")
}

fn is_echo_block_opener(code: &str) -> bool {
    let trimmed = code.trim().to_lowercase();
    trimmed.contains("begintag(") || trimmed.contains("::begin(")
}

fn is_echo_block_closer(code: &str) -> bool {
    let trimmed = code.trim().to_lowercase();
    trimmed.contains("endtag(") || trimmed.contains("::end(")
}

fn is_header_php_block(code: &str) -> bool {
    code.lines().any(|line| {
        let t = line.trim();
        t.starts_with("use ") || t.starts_with("declare(")
    })
}

fn is_void_element(name: &str) -> bool {
    VOID_ELEMENTS.contains(&name.to_lowercase().as_str())
}

fn format_attributes(attrs: &[Attribute]) -> String {
    if attrs.is_empty() {
        return String::new();
    }

    let parts: Vec<String> = attrs
        .iter()
        .map(|a| match &a.value {
            Some(v) => format!("{}=\"{}\"", a.name, v),
            None => a.name.clone(),
        })
        .collect();

    format!(" {}", parts.join(" "))
}

fn is_inline_content(children: &[Node]) -> bool {
    children.iter().all(|c| matches!(c, Node::Text(_) | Node::PhpEcho(_)))
}

fn format_inline(name: &str, attributes: &[Attribute], children: &[Node]) -> String {
    let attrs = format_attributes(attributes);
    let content: String = children
        .iter()
        .map(|c| match c {
            Node::Text(s) => s.trim().to_string(),
            Node::PhpEcho(s) => format!("<?= {} ?>", format_php_code(s)),
            _ => String::new(),
        })
        .collect();
    format!("<{name}{attrs}>{content}</{name}>")
}

fn count_leading_closers(s: &str) -> usize {
    s.chars().take_while(|c| matches!(c, ')' | ']' | '}')).count()
}

fn count_brackets(s: &str) -> (usize, usize) {
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

fn handle_comment_boundaries(chars: &[char], i: usize, ch: char, result: &mut String) {
    if ch == '/'
        && i + 1 < chars.len()
        && chars[i + 1] == '*'
        && i + 2 < chars.len()
        && chars[i + 2] == '*'
        && !result.ends_with('\n')
        && result.len() > 1
    {
        let last = result.pop().unwrap_or_default();
        result.push('\n');
        result.push(last);
    }
    if ch == '/' && result.len() >= 2 && result.ends_with("*/") {
        result.pop();
        result.pop();
        if !result.ends_with('\n') {
            let trimmed = result.trim_end().to_string();
            result.clear();
            result.push_str(&trimmed);
            result.push('\n');
        }
        result.push_str("*/\n");
        if i + 1 < chars.len() && chars[i + 1] != '\n' {
            result.push('\n');
        }
    }
    if ch == '*'
        && i + 1 < chars.len()
        && chars[i + 1] == ' '
        && i + 2 < chars.len()
        && chars[i + 2] == '@'
        && !result.ends_with('\n')
        && !result.ends_with('/')
    {
        result.pop();
        result.push('\n');
        result.push(ch);
    }
}

fn normalize_statements(code: &str) -> String {
    let mut result = String::from("\n");
    let chars: Vec<char> = code.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut paren_depth: i32 = 0;

    while i < len {
        let ch = chars[i];
        if ch == '\'' || ch == '"' {
            i = skip_string_literal(&chars, i, &mut result);
            continue;
        }
        if ch == '(' {
            paren_depth += 1;
        } else if ch == ')' {
            paren_depth -= 1;
        }

        if paren_depth <= 0
            && !result.ends_with('\n')
            && !result.trim_end().is_empty()
            && (matches_keyword_at(&chars, i, "case ") || matches_keyword_at(&chars, i, "default:"))
        {
            result.push('\n');
        }

        result.push(ch);
        if ch == ';' && paren_depth <= 0 && i + 1 < len && chars[i + 1] != '\n' {
            result.push('\n');
        }

        handle_comment_boundaries(&chars, i, ch, &mut result);
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

fn emit_reindented_line(formatted: &str, pad: &str, depth: &mut i32, result: &mut String) {
    let leading = count_leading_closers(formatted) as i32;
    let is_ternary_cont = formatted.starts_with("? ") || formatted.starts_with(": ");
    let extra = i32::from(is_ternary_cont);
    let write_depth = (*depth - leading + extra).max(0) as usize;
    let inner_pad = INDENT.repeat(write_depth);
    let base_pad = format!("{pad}{inner_pad}");
    if let Some(split) = try_split_long_line(formatted, &base_pad) {
        result.push_str(&split);
    } else if formatted.starts_with('*') {
        result.push_str(&format!("{pad}{inner_pad} {formatted}\n"));
    } else {
        result.push_str(&format!("{pad}{inner_pad}{formatted}\n"));
    }
    let (openers, closers) = count_brackets(formatted);
    let net = openers as i32 - closers as i32;
    *depth += net.min(1);
    *depth = (*depth).max(0);
}

fn join_ternary_lines(code: &str) -> String {
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
        i += 1;
        while i < lines.len() {
            let next = lines[i].trim();
            if next.is_empty() {
                break;
            }
            joined.push(' ');
            joined.push_str(next);
            i += 1;
            if next.contains(';') {
                break;
            }
        }
        result.push(joined);
    }

    result.join("\n")
}

fn reindent_php_block(code: &str, pad: &str) -> String {
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
    let mut prev_was_use = false;
    let mut prev_was_declare = false;
    let is_header = is_header_php_block(&code);

    for line in code.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !prev_blank && !first_content {
                result.push('\n');
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
        if (prev_was_use && !is_use_import && !prev_blank) || (prev_was_declare && !is_declare && !prev_blank) {
            result.push('\n');
        }
        if prev_was_doc_close && !prev_blank {
            result.push('\n');
        }
        prev_blank = false;
        prev_was_doc_close = trimmed == "*/";
        prev_was_use = is_use_import;
        prev_was_declare = is_declare;
        let formatted = format_php_code(trimmed);
        emit_reindented_line(&formatted, pad, &mut depth, &mut result);
    }

    result.trim_end_matches('\n').to_string() + "\n"
}

fn find_matching_close(chars: &[char], open_pos: usize) -> Option<usize> {
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

fn try_split_long_line(formatted: &str, base_pad: &str) -> Option<String> {
    if base_pad.len() + formatted.len() <= MAX_LINE_LENGTH {
        return None;
    }

    if let Some((q_pos, c_pos)) = find_ternary_positions(formatted) {
        let condition = formatted[..q_pos].trim_end();
        let true_val = formatted[q_pos + 1..c_pos].trim();
        let false_val = formatted[c_pos + 1..].trim();
        let inner_pad = format!("{base_pad}{INDENT}");
        return Some(format!(
            "{base_pad}{condition}\n{inner_pad}? {true_val}\n{inner_pad}: {false_val}\n"
        ));
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

fn build_split(prefix: &str, args: &[String], suffix: &str, pad: &str) -> String {
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
        }
        result.push_str(&format!("{inner_pad}{arg},\n"));
    }
    result.push_str(&format!("{pad}{suffix}\n"));
    result
}

fn expand_bare_array(arg: &str, pad: &str) -> Option<String> {
    let trimmed = arg.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return None;
    }
    let inner = &trimmed[1..trimmed.len() - 1];
    let items = split_by_commas(inner);
    if items.len() <= 1 {
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

fn expand_bare_sub_array(item: &str, pad: &str) -> Option<String> {
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
        result.push_str(&format!("{deeper_pad}{sub},\n"));
    }
    result.push_str(&format!("{pad}],\n"));
    Some(result)
}

fn find_array_arrow(arg: &str) -> Option<(usize, usize)> {
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

fn expand_nested_array(arg: &str, pad: &str) -> Option<String> {
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
        }
        result.push_str(&format!("{inner_pad}{item},\n"));
    }
    result.push_str(&format!("{pad}]{suffix} ?>\n"));
    Some(result)
}

fn find_ternary_positions(code: &str) -> Option<(usize, usize)> {
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

fn format_echo(code: &str, pad: &str) -> String {
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

    if let Some((prefix, args, suffix)) = split_by_args(&formatted) {
        if args.len() >= 2 {
            if let Some(r) = format_echo_array_split(&prefix, &args, &suffix, pad) {
                return r;
            }
        }
        let mut result = format!("{pad}<?= {prefix}\n");
        for arg in &args {
            result.push_str(&format!("{pad}{INDENT}{arg},\n"));
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

fn count_semicolons_outside_parens(code: &str) -> usize {
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

struct PhpDepthState {
    depth: usize,
    switch_stack: Vec<usize>,
}

fn emit_element(name: &str, attributes: &[Attribute], children: &[Node], ctx: (usize, &mut String)) {
    let (depth, output) = ctx;
    let pad = INDENT.repeat(depth);
    let attrs = format_attributes(attributes);
    if RAW_TEXT_ELEMENTS.contains(&name.to_lowercase().as_str()) {
        output.push_str(&format!("{pad}<{name}{attrs}>\n"));
        for child in children {
            if let Node::Text(s) = child {
                let trimmed = s.trim_start_matches('\n').trim_end();
                if !trimmed.is_empty() {
                    for line in trimmed.lines() {
                        if line.chars().next().is_some_and(char::is_whitespace) {
                            output.push_str(line);
                        } else {
                            output.push_str(&pad);
                            output.push_str(line);
                        }
                        output.push('\n');
                    }
                }
            }
        }
        output.push_str(&format!("{pad}</{name}>\n"));
    } else if children.is_empty() && is_void_element(name) {
        output.push_str(&format!("{pad}<{name}{attrs} />\n"));
    } else if is_inline_content(children) {
        let inline = format_inline(name, attributes, children);
        if pad.len() + inline.len() <= MAX_LINE_LENGTH {
            output.push_str(&pad);
            output.push_str(&inline);
            output.push('\n');
        } else {
            output.push_str(&format!("{pad}<{name}{attrs}>\n"));
            format_nodes(children, depth + 1, output);
            output.push_str(&format!("{pad}</{name}>\n"));
        }
    } else {
        output.push_str(&format!("{pad}<{name}{attrs}>\n"));
        format_nodes(children, depth + 1, output);
        output.push_str(&format!("{pad}</{name}>\n"));
    }
}

fn emit_switch_stmt(trimmed: &str, state: &mut PhpDepthState, output: &mut String) {
    let formatted = format_php_code(trimmed);
    let lower = trimmed.to_lowercase();
    if lower.starts_with("switch") && is_php_block_opener(trimmed) {
        let pad = INDENT.repeat(state.depth);
        output.push_str(&format!("{pad}<?php {formatted} ?>\n"));
        state.switch_stack.push(state.depth);
        state.depth += 1;
    } else if is_switch_case_peer(trimmed) {
        let lvl = state
            .switch_stack
            .last()
            .copied()
            .unwrap_or(state.depth.saturating_sub(1));
        let pad = INDENT.repeat(lvl);
        output.push_str(&format!("{pad}<?php {formatted} ?>\n"));
        state.depth = lvl + 1;
    } else if lower.starts_with("endswitch") {
        let lvl = state.switch_stack.pop().unwrap_or(state.depth.saturating_sub(1));
        let pad = INDENT.repeat(lvl);
        output.push_str(&format!("{pad}<?php {formatted} ?>\n"));
        state.depth = lvl;
    } else if contains_outside_strings(&lower, "break;") {
        let pad = INDENT.repeat(state.depth);
        output.push_str(&format!("{pad}<?php {formatted} ?>\n"));
    } else {
        let pad = INDENT.repeat(state.depth);
        output.push_str(&format!("{pad}<?php {formatted} ?>\n"));
        if is_php_block_opener(trimmed) {
            state.depth += 1;
        }
    }
}

fn emit_multiline_php(code: &str, pad: &str, depth: &mut usize, output: &mut String) {
    let is_header = is_header_php_block(code);
    if is_header {
        output.push_str(&format!("{pad}<?php\n"));
        let reindented = reindent_php_block(code, pad);
        output.push_str(&reindented);
        output.push('\n');
        output.push_str(&format!("{pad}?>\n"));
    } else {
        emit_multiline_php_inline(code, pad, output);
    }
    let has_widget_pair = code.contains("::begin(") || code.contains("::end(");
    if has_widget_pair || !is_header {
        if is_php_block_closer(code) && has_widget_pair {
            *depth = depth.saturating_sub(1);
        } else if (has_widget_pair || !is_php_block_closer(code)) && is_php_block_opener(code) {
            *depth += 1;
        }
    }
}

fn emit_multiline_php_inline(code: &str, pad: &str, output: &mut String) {
    let reindented = reindent_php_block(code, pad);
    let lines: Vec<&str> = reindented.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.len() > 1 {
        output.push_str(&format!("{pad}<?php {}\n", lines[0].trim_start()));
        for line in &lines[1..lines.len() - 1] {
            output.push_str(line);
            output.push('\n');
        }
        output.push_str(&format!("{} ?>\n", lines[lines.len() - 1]));
    } else if lines.len() == 1 {
        output.push_str(&format!("{pad}<?php {} ?>\n", lines[0].trim_start()));
    }
}

fn emit_single_php(code: &str, pad: &str, state: &mut PhpDepthState, output: &mut String) {
    let formatted = format_php_code(code);
    let lower = code.trim().to_lowercase();
    if lower.starts_with("switch") && is_php_block_opener(code) && !is_php_block_closer(code) {
        let stmt_pad = INDENT.repeat(state.depth);
        output.push_str(&format!("{stmt_pad}<?php {formatted} ?>\n"));
        state.switch_stack.push(state.depth);
        state.depth += 1;
    } else if !state.switch_stack.is_empty() && is_switch_case_peer(code) {
        let lvl = state
            .switch_stack
            .last()
            .copied()
            .unwrap_or(state.depth.saturating_sub(1));
        let stmt_pad = INDENT.repeat(lvl);
        output.push_str(&format!("{stmt_pad}<?php {formatted} ?>\n"));
        state.depth = lvl + 1;
    } else if !state.switch_stack.is_empty() && lower.starts_with("endswitch") {
        let lvl = state.switch_stack.pop().unwrap_or(state.depth.saturating_sub(1));
        let stmt_pad = INDENT.repeat(lvl);
        output.push_str(&format!("{stmt_pad}<?php {formatted} ?>\n"));
        state.depth = lvl;
    } else if !state.switch_stack.is_empty() && contains_outside_strings(&lower, "break;") {
        let stmt_pad = INDENT.repeat(state.depth);
        output.push_str(&format!("{stmt_pad}<?php {formatted} ?>\n"));
    } else if is_php_block_closer(code) {
        state.depth = state.depth.saturating_sub(1);
        let pad_less = INDENT.repeat(state.depth);
        output.push_str(&format!("{pad_less}<?php {formatted} ?>\n"));
        if is_php_block_opener(code) {
            state.depth += 1;
        }
    } else {
        emit_single_php_long(code, pad, &mut state.depth, output);
    }
}

fn emit_single_php_long(code: &str, pad: &str, depth: &mut usize, output: &mut String) {
    let formatted = format_php_code(code);
    let single = format!("{pad}<?php {formatted} ?>");
    if single.len() <= MAX_LINE_LENGTH {
        output.push_str(&format!("{single}\n"));
    } else if let Some((q_pos, c_pos)) = find_ternary_positions(&formatted) {
        let condition = formatted[..q_pos].trim_end();
        let true_val = formatted[q_pos + 1..c_pos].trim();
        let false_val = formatted[c_pos + 1..].trim();
        let inner_pad = format!("{pad}{INDENT}");
        output.push_str(&format!(
            "{pad}<?php {condition}\n{inner_pad}? {true_val}\n{inner_pad}: {false_val} ?>\n"
        ));
    } else {
        let reindented = reindent_php_block(code, pad);
        let lines: Vec<&str> = reindented.lines().filter(|l| !l.trim().is_empty()).collect();
        if lines.len() > 1 {
            output.push_str(&format!("{pad}<?php {}\n", lines[0].trim_start()));
            for line in &lines[1..lines.len() - 1] {
                output.push_str(line);
                output.push('\n');
            }
            output.push_str(&format!("{} ?>\n", lines[lines.len() - 1]));
        } else {
            output.push_str(&format!("{pad}<?php\n"));
            output.push_str(&reindented);
            output.push_str(&format!("{pad}?>\n"));
        }
    }
    if is_php_block_opener(code) {
        *depth += 1;
    }
}

fn emit_php_block(code: &str, pad: &str, state: &mut PhpDepthState, output: &mut String) {
    let semicolons = count_semicolons_outside_parens(code);
    let is_multiline = code.contains('\n') || semicolons > 1 || has_switch_case(code);
    if is_multiline && has_switch_case(code) {
        for stmt_line in normalize_statements(code).lines() {
            let trimmed = stmt_line.trim();
            if !trimmed.is_empty() {
                emit_switch_stmt(trimmed, state, output);
            }
        }
    } else if is_multiline {
        emit_multiline_php(code, pad, &mut state.depth, output);
    } else {
        emit_single_php(code, pad, state, output);
    }
}

fn emit_php_echo(code: &str, pad: &str, state: &mut PhpDepthState, output: &mut String) {
    if is_echo_block_closer(code) {
        state.depth = state.depth.saturating_sub(1);
        let pad = INDENT.repeat(state.depth);
        output.push_str(&format_echo(code, &pad));
    } else {
        output.push_str(&format_echo(code, pad));
        if is_echo_block_opener(code) {
            state.depth += 1;
        }
    }
}

fn format_nodes(nodes: &[Node], depth: usize, output: &mut String) {
    let mut state = PhpDepthState {
        depth,
        switch_stack: Vec::new(),
    };

    for node in nodes {
        let pad = INDENT.repeat(state.depth);

        match node {
            Node::Element {
                name,
                attributes,
                children,
            } => {
                emit_element(name, attributes, children, (state.depth, output));
            }
            Node::Text(s) => {
                let trimmed = s.trim();
                if !trimmed.is_empty() {
                    output.push_str(&format!("{pad}{trimmed}\n"));
                } else if state.depth <= 1 && s.contains('\n') && s.chars().filter(|&c| c == '\n').count() > 1 {
                    output.push('\n');
                }
            }
            Node::PhpBlock(code) => emit_php_block(code, &pad, &mut state, output),
            Node::PhpEcho(code) => emit_php_echo(code, &pad, &mut state, output),
            Node::Doctype(s) => {
                output.push_str(&format!("{pad}<!DOCTYPE {s}>\n"));
            }
            Node::Comment(s) => {
                output.push_str(&format!("{pad}<!-- {s} -->\n"));
            }
        }
    }
}

pub fn format(nodes: &[Node]) -> String {
    let mut output = String::new();
    format_nodes(nodes, 0, &mut output);
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{ast, lexer};
    use pretty_assertions::assert_eq;

    fn format_str(input: &str) -> String {
        let tokens = lexer::tokenize(input);
        let nodes = ast::parse(tokens);
        format(&nodes)
    }

    #[test]
    fn simple_div() {
        assert_eq!(format_str("<div>hello</div>"), "<div>hello</div>\n");
    }

    #[test]
    fn nested_html() {
        let input = "<div><p>text</p></div>";
        let expected = "\
<div>
    <p>text</p>
</div>
";
        assert_eq!(format_str(input), expected);
    }

    #[test]
    fn self_closing_tag() {
        assert_eq!(format_str("<br />"), "<br />\n");
    }

    #[test]
    fn php_echo_inline() {
        let input = "<h1><?= $title ?></h1>";
        assert_eq!(format_str(input), "<h1><?= $title ?></h1>\n");
    }

    #[test]
    fn php_block_indentation() {
        let input = "<div><?php if ($x): ?><p>yes</p><?php endif; ?></div>";
        let expected = "\
<div>
    <?php if ($x): ?>
        <p>yes</p>
    <?php endif; ?>
</div>
";
        assert_eq!(format_str(input), expected);
    }

    #[test]
    fn attributes_preserved() {
        let input = r#"<div class="container" id="main"><p>hi</p></div>"#;
        let expected = "\
<div class=\"container\" id=\"main\">
    <p>hi</p>
</div>
";
        assert_eq!(format_str(input), expected);
    }

    #[test]
    fn nested_php_blocks() {
        let input = "<div><?php if ($a): ?><?php foreach ($items as $i): ?><p><?= $i ?></p><?php endforeach; ?><?php endif; ?></div>";
        let expected = "\
<div>
    <?php if ($a): ?>
        <?php foreach ($items as $i): ?>
            <p><?= $i ?></p>
        <?php endforeach; ?>
    <?php endif; ?>
</div>
";
        assert_eq!(format_str(input), expected);
    }
}
