use super::Formatter;
use super::indent::visual_len;
use super::php::{format_php_code, join_php_lines};
use crate::parser::lexer::Attribute;

pub(crate) fn format_attributes(attrs: &[Attribute]) -> String {
    if attrs.is_empty() {
        return String::new();
    }

    let parts: Vec<String> = attrs.iter().map(format_attribute).collect();
    format!(" {}", parts.join(" "))
}

fn attr_quote(value: &str) -> char {
    if value.contains('"') && !value.contains('\'') {
        '\''
    } else {
        '"'
    }
}

fn normalize_php_segment(seg: &str) -> String {
    let Some(inner) = seg.strip_suffix("?>") else {
        return seg.to_string();
    };
    if let Some(rest) = inner.strip_prefix("<?=") {
        format!("<?= {} ?>", format_php_code(&join_php_lines(rest.trim())))
    } else if let Some(rest) = inner.strip_prefix("<?php") {
        format!("<?php {} ?>", format_php_code(&join_php_lines(rest.trim())))
    } else {
        seg.to_string()
    }
}

fn find_php_close_tag(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut i = 0;
    let mut in_single = false;
    let mut in_double = false;
    let mut in_block_comment = false;
    while i + 1 < bytes.len() {
        let b = bytes[i];
        if in_block_comment {
            if b == b'*' && bytes[i + 1] == b'/' {
                in_block_comment = false;
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }
        if (in_single || in_double) && b == b'\\' {
            i += 2;
            continue;
        }
        match b {
            b'\'' if !in_double => in_single = !in_single,
            b'"' if !in_single => in_double = !in_double,
            b'/' if !in_single && !in_double && bytes[i + 1] == b'*' => {
                in_block_comment = true;
                i += 2;
                continue;
            }
            b'?' if !in_single && !in_double && bytes[i + 1] == b'>' => return Some(i),
            _ => {}
        }
        i += 1;
    }
    None
}

fn normalize_attr_value(value: &str) -> String {
    if !value.contains("<?") {
        return value.to_string();
    }
    let mut out = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(start) = rest.find("<?") {
        out.push_str(&rest[..start]);
        let after = &rest[start..];
        let Some(close) = find_php_close_tag(after) else {
            out.push_str(after);
            return out;
        };
        out.push_str(&normalize_php_segment(&after[..close + 2]));
        rest = &after[close + 2..];
    }
    out.push_str(rest);
    out
}

fn single_php_segment(value: &str) -> Option<(String, bool, String, String)> {
    let start = value.find("<?")?;
    let after = &value[start..];
    let is_echo = after.starts_with("<?=");
    let is_php = after.starts_with("<?php");
    if !is_echo && !is_php {
        return None;
    }
    let close = find_php_close_tag(after)?;
    let suffix = &after[close + 2..];
    if suffix.contains("<?") {
        return None;
    }
    let inner = &after[..close];
    let code = if is_echo {
        inner.strip_prefix("<?=")?.trim()
    } else {
        inner.strip_prefix("<?php")?.trim()
    };
    Some((
        value[..start].to_string(),
        is_echo,
        code.to_string(),
        suffix.to_string(),
    ))
}

fn format_attribute(attr: &Attribute) -> String {
    let Some(value) = &attr.value else {
        return attr.name.clone();
    };
    let value = normalize_attr_value(value);
    let quote = attr_quote(&value);
    format!("{}={quote}{value}{quote}", attr.name)
}

impl Formatter {
    pub(crate) fn emit_open_tag(&self, name: &str, attributes: &[Attribute], pad: &str, output: &mut String) {
        let attrs = format_attributes(attributes);
        let single = format!("{pad}<{name}{attrs}>");

        if attributes.is_empty() || visual_len(&single) <= self.max_line_length {
            output.push_str(&single);
            output.push('\n');
            return;
        }

        let indent = &self.indent;
        output.push_str(&format!("{pad}<{name}\n"));
        let attr_pad = format!("{pad}{indent}");
        for attr in attributes {
            let line = format!("{attr_pad}{}", format_attribute(attr));
            if visual_len(&line) <= self.max_line_length || !self.emit_attribute_split(attr, &attr_pad, output) {
                output.push_str(&line);
                output.push('\n');
            }
        }
        output.push_str(&format!("{pad}>\n"));
    }

    fn emit_attribute_split(&self, attr: &Attribute, attr_pad: &str, output: &mut String) -> bool {
        let Some(raw) = &attr.value else {
            return false;
        };
        let value = normalize_attr_value(raw);
        let Some((prefix, is_echo, code, suffix)) = single_php_segment(&value) else {
            return false;
        };
        let Some(split) = self.try_split_long_line(&code, attr_pad) else {
            return false;
        };
        let lines: Vec<&str> = split.lines().filter(|l| !l.trim().is_empty()).collect();
        if lines.len() < 2 {
            return false;
        }
        let quote = attr_quote(&value);
        let open = if is_echo { "<?=" } else { "<?php" };
        output.push_str(&format!(
            "{attr_pad}{}={quote}{prefix}{open} {}\n",
            attr.name,
            lines[0].trim_start()
        ));
        for line in &lines[1..lines.len() - 1] {
            output.push_str(line);
            output.push('\n');
        }
        output.push_str(&format!("{} ?>{suffix}{quote}\n", lines[lines.len() - 1]));
        true
    }
}
