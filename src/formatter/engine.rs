use super::php::{format_php_code, join_php_lines, split_by_args, split_by_chain, split_by_commas};
use crate::parser::ast::Node;
use crate::parser::lexer::Attribute;

const INDENT: &str = "    ";
const MAX_LINE_LENGTH: usize = 120;

const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source", "track", "wbr",
];

fn is_php_block_opener(code: &str) -> bool {
    code.trim().ends_with(':')
}

fn is_php_block_closer(code: &str) -> bool {
    let lower = code.trim().to_lowercase();
    lower.starts_with("endif")
        || lower.starts_with("endforeach")
        || lower.starts_with("endfor")
        || lower.starts_with("endwhile")
        || lower.starts_with("else")
        || lower.starts_with("elseif")
}

fn is_echo_block_opener(code: &str) -> bool {
    let trimmed = code.trim().to_lowercase();
    trimmed.contains("begintag(")
}

fn is_echo_block_closer(code: &str) -> bool {
    let trimmed = code.trim().to_lowercase();
    trimmed.contains("endtag(")
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
    children.len() <= 2 && children.iter().all(|c| matches!(c, Node::Text(_) | Node::PhpEcho(_)))
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

fn normalize_statements(code: &str) -> String {
    let mut result = String::from("\n");
    let chars: Vec<char> = code.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let ch = chars[i];
        if ch == '\'' || ch == '"' {
            result.push(ch);
            i += 1;
            while i < len && chars[i] != ch {
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
            continue;
        }
        result.push(ch);
        if ch == ';' && i + 1 < len && chars[i + 1] != '\n' {
            result.push('\n');
        }

        if ch == '/'
            && i + 1 < len
            && chars[i + 1] == '*'
            && i + 2 < len
            && chars[i + 2] == '*'
            && !result.ends_with('\n')
            && result.len() > 1
        {
            let last = result.pop().unwrap();
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
            if i + 1 < len && chars[i + 1] != '\n' {
                result.push('\n');
            }
        }

        if ch == '*'
            && i + 1 < len
            && chars[i + 1] == ' '
            && i + 2 < len
            && chars[i + 2] == '@'
            && !result.ends_with('\n')
            && !result.ends_with('/')
        {
            result.pop();
            result.push('\n');
            result.push(ch);
        }
        i += 1;
    }

    result
}

fn reindent_php_block(code: &str, pad: &str) -> String {
    let code = if !code.contains('\n') && code.contains(';') {
        normalize_statements(code)
    } else {
        code.to_string()
    };
    let mut result = String::new();
    let mut depth: i32 = 0;
    let mut prev_blank = false;
    let mut first_content = true;
    let mut prev_was_doc_close = false;
    let mut prev_was_use = false;
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
        if prev_was_use && !is_use_import && !prev_blank {
            result.push('\n');
        }

        if prev_was_doc_close && !prev_blank {
            result.push('\n');
        }

        prev_blank = false;
        prev_was_doc_close = trimmed == "*/";
        prev_was_use = is_use_import;

        let formatted = format_php_code(trimmed);
        let leading = count_leading_closers(&formatted) as i32;
        let write_depth = (depth - leading).max(0) as usize;
        let inner_pad = INDENT.repeat(write_depth);
        let base_pad = format!("{pad}{inner_pad}");

        if let Some(split) = try_split_long_line(&formatted, &base_pad) {
            result.push_str(&split);
        } else if formatted.starts_with('*') {
            result.push_str(&format!("{pad}{inner_pad} {formatted}\n"));
        } else {
            result.push_str(&format!("{pad}{inner_pad}{formatted}\n"));
        }

        let (openers, closers) = count_brackets(&formatted);
        let net = openers as i32 - closers as i32;
        depth += net.min(1);
        depth = depth.max(0);
    }

    result
}

fn try_split_long_line(formatted: &str, base_pad: &str) -> Option<String> {
    if base_pad.len() + formatted.len() <= MAX_LINE_LENGTH {
        return None;
    }

    if let Some((prefix, args, suffix)) = split_by_args(formatted) {
        return Some(build_split(&prefix, &args, &suffix, base_pad));
    }

    let chars: Vec<char> = formatted.chars().collect();
    let len = chars.len();
    let open_pos = chars.iter().position(|&c| c == '(')?;

    let mut depth = 0i32;
    let mut close_pos = None;
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
        } else if matches!(ch, '(' | '[') {
            depth += 1;
        } else if matches!(ch, ')' | ']') {
            depth -= 1;
            if depth == 0 {
                close_pos = Some(i);
                break;
            }
        }
        i += 1;
    }

    let close_pos = close_pos?;
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
        if item_line_len > MAX_LINE_LENGTH
            && let Some(expanded) = expand_nested_array(item, &nested_pad)
        {
            result.push_str(&expanded);
            continue;
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

fn expand_nested_array(arg: &str, pad: &str) -> Option<String> {
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

    let rest = &arg[i..];
    let arrow_in_rest = rest.find(" => ")?;
    let value_start = i + arrow_in_rest + 4;

    let value = arg[value_start..].trim();
    if !value.starts_with('[') || !value.ends_with(']') {
        return None;
    }

    let inner = &value[1..value.len() - 1];
    let items = split_by_commas(inner);
    if items.len() <= 1 {
        return None;
    }

    let key = &arg[..value_start];
    let nested_pad = format!("{pad}{INDENT}");
    let mut result = format!("{pad}{key}[\n");
    for item in &items {
        let item_line_len = nested_pad.len() + item.len() + 1;
        if item_line_len > MAX_LINE_LENGTH
            && let Some(expanded) = expand_nested_array(item, &nested_pad)
        {
            result.push_str(&expanded);
            continue;
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

fn format_echo(code: &str, pad: &str) -> String {
    let joined = join_php_lines(code);
    let formatted = format_php_code(&joined);
    let single = format!("{pad}<?= {formatted} ?>");

    if single.len() <= MAX_LINE_LENGTH {
        return format!("{single}\n");
    }

    let parts = split_by_chain(&formatted);
    if parts.len() > 2 {
        let chain_pad = format!("{pad}{INDENT}");
        let mut result = format!("{pad}<?= {}{}", parts[0], parts[1]);
        for part in &parts[2..] {
            let part_line_len = chain_pad.len() + part.len();
            if part_line_len > MAX_LINE_LENGTH
                && let Some(split) = try_split_long_line(part, &chain_pad)
            {
                let split_content = split.trim_start();
                result.push_str(&format!("\n{chain_pad}{split_content}"));
                continue;
            }
            result.push_str(&format!("\n{chain_pad}{part}"));
        }
        result.push_str(" ?>");
        result.push('\n');
        return result;
    }

    if let Some((prefix, args, suffix)) = split_by_args(&formatted) {
        // Smart inline: keep short args inline, expand only the last array arg
        if args.len() >= 2 {
            let last = &args[args.len() - 1];
            if last.starts_with('[') && last.ends_with(']') {
                let inline_parts: Vec<&str> = args[..args.len() - 1].iter().map(|s| s.as_str()).collect();
                let inline_joined = inline_parts.join(", ");
                let inline_prefix = format!("{pad}<?= {prefix}{inline_joined}, [");
                if inline_prefix.len() <= MAX_LINE_LENGTH {
                    let inner = &last[1..last.len() - 1];
                    let items = split_by_commas(inner);
                    if items.len() > 1 {
                        let inner_pad = format!("{pad}{INDENT}");
                        let mut result = format!("{pad}<?= {prefix}{inline_joined}, [\n");
                        for item in &items {
                            let item_line_len = inner_pad.len() + item.len() + 1;
                            if item_line_len > MAX_LINE_LENGTH
                                && let Some(expanded) = expand_nested_array(item, &inner_pad)
                            {
                                result.push_str(&expanded);
                                continue;
                            }
                            result.push_str(&format!("{inner_pad}{item},\n"));
                        }
                        result.push_str(&format!("{pad}]{suffix} ?>\n"));
                        return result;
                    }
                }
            }
        }

        // Full split: each arg on its own line
        let mut result = format!("{pad}<?= {prefix}\n");
        for arg in &args {
            result.push_str(&format!("{pad}{INDENT}{arg},\n"));
        }
        result.push_str(&format!("{pad}{suffix} ?>\n"));
        return result;
    }

    if let Some(split) = try_split_long_line(&formatted, pad) {
        return format!("{}<?= ", pad) + split.trim_start() + " ?>\n";
    }

    format!("{single}\n")
}

fn format_nodes(nodes: &[Node], depth: usize, output: &mut String) {
    let mut current_depth = depth;

    for node in nodes {
        let pad = INDENT.repeat(current_depth);

        match node {
            Node::Element {
                name,
                attributes,
                children,
            } => {
                let attrs = format_attributes(attributes);

                if children.is_empty() && is_void_element(name) {
                    output.push_str(&format!("{pad}<{name}{attrs} />\n"));
                } else if is_inline_content(children) {
                    let inline = format_inline(name, attributes, children);
                    if pad.len() + inline.len() <= MAX_LINE_LENGTH {
                        output.push_str(&pad);
                        output.push_str(&inline);
                        output.push('\n');
                    } else {
                        let attrs = format_attributes(attributes);
                        output.push_str(&format!("{pad}<{name}{attrs}>\n"));
                        format_nodes(children, current_depth + 1, output);
                        output.push_str(&format!("{pad}</{name}>\n"));
                    }
                } else {
                    output.push_str(&format!("{pad}<{name}{attrs}>\n"));
                    format_nodes(children, current_depth + 1, output);
                    output.push_str(&format!("{pad}</{name}>\n"));
                }
            }
            Node::Text(s) => {
                let trimmed = s.trim();
                if !trimmed.is_empty() {
                    output.push_str(&format!("{pad}{trimmed}\n"));
                } else if current_depth <= 1 && s.contains('\n') && s.chars().filter(|&c| c == '\n').count() > 1 {
                    output.push('\n');
                }
            }
            Node::PhpBlock(code) => {
                let is_multiline = code.contains('\n') || code.chars().filter(|&c| c == ';').count() > 1;
                if is_multiline {
                    let is_header = is_header_php_block(code);
                    if is_header {
                        output.push_str(&format!("{pad}<?php\n"));
                        output.push_str(&reindent_php_block(code, &pad));
                        output.push('\n');
                        output.push_str(&format!("{pad}?>\n"));
                    } else {
                        let reindented = reindent_php_block(code, &pad);
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
                } else {
                    let formatted = format_php_code(code);
                    if is_php_block_closer(code) {
                        current_depth = current_depth.saturating_sub(1);
                        let pad_less = INDENT.repeat(current_depth);
                        output.push_str(&format!("{pad_less}<?php {formatted} ?>\n"));
                    } else {
                        let single = format!("{pad}<?php {formatted} ?>");
                        if single.len() <= MAX_LINE_LENGTH {
                            output.push_str(&format!("{single}\n"));
                        } else {
                            let reindented = reindent_php_block(code, &pad);
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
                            current_depth += 1;
                        }
                    }
                }
            }
            Node::PhpEcho(code) => {
                if is_echo_block_closer(code) {
                    current_depth = current_depth.saturating_sub(1);
                    let pad = INDENT.repeat(current_depth);
                    output.push_str(&format_echo(code, &pad));
                } else {
                    output.push_str(&format_echo(code, &pad));
                    if is_echo_block_opener(code) {
                        current_depth += 1;
                    }
                }
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
