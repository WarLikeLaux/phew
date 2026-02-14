use super::indent::emit_reindented_line;

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

pub fn flush_docblocks(bodies: &[String], pad: &str, depth: &mut i32, result: &mut String) {
    let merged = if bodies.len() == 1 {
        format!("/**\n * {}\n */", bodies[0])
    } else {
        merge_docblock_bodies(bodies)
    };
    for doc_line in merged.lines() {
        emit_reindented_line(doc_line, pad, depth, result);
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
    if lines.first().copied() != Some("/**") || lines.last().copied() != Some("*/") {
        return false;
    }

    lines[1..lines.len() - 1].iter().all(|line| line.starts_with('*'))
}

pub fn emit_docblock_php(code: &str, pad: &str, output: &mut String) {
    let docblock = expand_single_line_docblock(code).unwrap_or_else(|| code.trim().to_string());
    output.push_str(&format!("{pad}<?php\n"));
    for line in docblock.lines() {
        output.push_str(pad);
        output.push_str(line.trim_end());
        output.push('\n');
    }
    output.push_str(&format!("{pad}?>\n"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_single_line() {
        assert_eq!(
            expand_single_line_docblock("/** @var string $x */"),
            Some("/**\n * @var string $x\n */".to_string())
        );
    }

    #[test]
    fn expand_empty_docblock() {
        assert_eq!(expand_single_line_docblock("/** */"), Some("/**\n */".to_string()));
    }

    #[test]
    fn extract_body() {
        assert_eq!(
            extract_docblock_body("/** @var string $x */"),
            Some("@var string $x".to_string())
        );
    }

    #[test]
    fn extract_empty_returns_none() {
        assert_eq!(extract_docblock_body("/** */"), None);
    }

    #[test]
    fn merge_single_body() {
        assert_eq!(
            merge_docblock_bodies(&["@var string $x".to_string()]),
            "/**\n * @var string $x\n */"
        );
    }

    #[test]
    fn merge_with_empty_separator() {
        let bodies = vec!["Description".to_string(), String::new(), "@var string $x".to_string()];
        assert_eq!(
            merge_docblock_bodies(&bodies),
            "/**\n * Description\n *\n * @var string $x\n */"
        );
    }

    #[test]
    fn descriptions_and_vars_with_separator() {
        let descs = vec!["Hello".to_string()];
        let vars = vec!["@var int $x".to_string()];
        let result = merge_descriptions_and_vars(&descs, &vars);
        assert_eq!(result, vec!["Hello", "", "@var int $x"]);
    }

    #[test]
    fn docblock_only_single_line() {
        assert!(is_docblock_only("/** @var string $x */"));
    }

    #[test]
    fn docblock_only_multiline() {
        assert!(is_docblock_only("/**\n * @var string $x\n */"));
    }

    #[test]
    fn not_docblock_only() {
        assert!(!is_docblock_only("$x = 1;"));
    }

    #[test]
    fn normalize_var_reversed_order() {
        assert_eq!(normalize_var_body("@var $model User"), "@var User $model");
    }

    #[test]
    fn normalize_var_correct_order_unchanged() {
        assert_eq!(normalize_var_body("@var User $model"), "@var User $model");
    }

    #[test]
    fn normalize_var_non_var_unchanged() {
        assert_eq!(normalize_var_body("@return string"), "@return string");
    }

    #[test]
    fn extract_body_normalizes_var_order() {
        assert_eq!(
            extract_docblock_body("/** @var $this yii\\web\\View */"),
            Some("@var yii\\web\\View $this".to_string())
        );
    }
}
