use super::Formatter;

fn normalize_var_body(body: &str) -> String {
    if !body.starts_with("@var ") {
        return body.to_string();
    }
    let rest = body[5..].trim();
    let parts: Vec<&str> = rest.splitn(3, ' ').collect();
    if parts.len() >= 2 && parts[0].starts_with('$') && !parts[1].starts_with('$') {
        let var_name = parts[0];
        let type_name = parts[1];
        return if parts.len() == 3 {
            format!("@var {type_name} {var_name} {}", parts[2])
        } else {
            format!("@var {type_name} {var_name}")
        };
    }
    body.to_string()
}

pub fn expand_single_line_docblock(code: &str) -> Option<String> {
    let trimmed = code.trim();
    if trimmed.contains('\n') || !trimmed.starts_with("/**") || !trimmed.ends_with("*/") {
        return None;
    }

    let mut body = trimmed.strip_prefix("/**").and_then(|s| s.strip_suffix("*/"))?.trim();
    if let Some(rest) = body.strip_prefix('*') {
        body = rest.trim_start();
    }

    if body.is_empty() {
        Some("/**\n */".to_string())
    } else {
        Some(format!("/**\n * {body}\n */"))
    }
}

pub fn extract_docblock_body(code: &str) -> Option<String> {
    let trimmed = code.trim();
    if trimmed.contains('\n') || !trimmed.starts_with("/**") || !trimmed.ends_with("*/") {
        return None;
    }
    let mut body = trimmed.strip_prefix("/**").and_then(|s| s.strip_suffix("*/"))?.trim();
    if let Some(rest) = body.strip_prefix('*') {
        body = rest.trim_start();
    }
    if body.is_empty() {
        return None;
    }
    Some(normalize_var_body(body))
}

pub fn merge_docblock_bodies(bodies: &[String]) -> String {
    let mut result = String::from("/**");
    for body in bodies {
        if body.is_empty() {
            result.push_str("\n *");
        } else {
            result.push_str(&format!("\n * {body}"));
        }
    }
    result.push_str("\n */");
    result
}

pub fn merge_descriptions_and_vars(descriptions: &[String], vars: &[String]) -> Vec<String> {
    let mut all_bodies: Vec<String> = Vec::new();
    all_bodies.extend_from_slice(descriptions);
    if !descriptions.is_empty() && !vars.is_empty() {
        all_bodies.push(String::new());
    }
    all_bodies.extend_from_slice(vars);
    all_bodies
}

impl Formatter {
    pub(crate) fn flush_docblocks(&self, bodies: &[String], pad: &str, depth: &mut i32, result: &mut String) {
        let merged = if bodies.len() == 1 {
            format!("/**\n * {}\n */", bodies[0])
        } else {
            merge_docblock_bodies(bodies)
        };
        for doc_line in merged.lines() {
            self.emit_reindented_line(doc_line, pad, depth, result);
        }
    }
}

pub fn is_docblock_only(code: &str) -> bool {
    let trimmed = code.trim();
    if trimmed.is_empty() {
        return false;
    }

    if trimmed.starts_with("/**") && trimmed.ends_with("*/") && !trimmed.contains('\n') {
        return true;
    }

    let lines: Vec<&str> = trimmed.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    if lines.len() < 2 {
        return false;
    }
    let last = lines.last().copied().unwrap_or("");
    if lines.first().copied() != Some("/**") || (last != "*/" && last != "**/") {
        return false;
    }

    lines[1..lines.len() - 1].iter().all(|line| line.starts_with('*'))
}

pub fn emit_docblock_php(code: &str, pad: &str, output: &mut String) {
    let docblock = expand_single_line_docblock(code).unwrap_or_else(|| code.trim().to_string());
    output.push_str(&format!("{pad}<?php\n"));
    for line in docblock.lines() {
        output.push_str(pad);
        output.push_str(&normalize_docblock_line(line));
        output.push('\n');
    }
    output.push_str(&format!("{pad}?>\n"));
}

fn normalize_docblock_line(line: &str) -> String {
    let trimmed = line.trim();
    if trimmed == "/**" {
        return trimmed.to_string();
    }
    if trimmed == "*/" || trimmed == "**/" {
        return " */".to_string();
    }
    if let Some(body) = trimmed.strip_prefix('*') {
        let body = body.trim_start();
        return if body.is_empty() {
            " *".to_string()
        } else {
            format!(" * {body}")
        };
    }
    trimmed.to_string()
}

#[cfg(test)]
#[path = "docblock_tests.rs"]
mod tests;
