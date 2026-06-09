#[derive(Clone, Copy, PartialEq, Eq)]
enum DeclKind {
    ClassLike,
    Function,
}

const DECL_MODIFIERS: &[&str] = &[
    "public",
    "private",
    "protected",
    "static",
    "final",
    "abstract",
    "readonly",
];
const CLASS_KEYWORDS: &[&str] = &["class", "interface", "trait", "enum"];

fn leading_word(s: &str) -> (&str, &str) {
    let trimmed = s.trim_start();
    let end = trimmed
        .find(|c: char| !(c.is_alphanumeric() || c == '_'))
        .unwrap_or(trimmed.len());
    (&trimmed[..end], trimmed[end..].trim_start())
}

fn decl_kind(header: &str) -> Option<DeclKind> {
    let mut rest = header;
    loop {
        let (word, after) = leading_word(rest);
        if word.is_empty() {
            return None;
        }
        if DECL_MODIFIERS.contains(&word) {
            rest = after;
            continue;
        }
        if CLASS_KEYWORDS.contains(&word) {
            return Some(DeclKind::ClassLike);
        }
        if word == "function" {
            let named = after.starts_with(|c: char| c.is_alphabetic() || c == '_' || c == '&');
            return named.then_some(DeclKind::Function);
        }
        return None;
    }
}

fn skip_leading_docblock(code: &str) -> &str {
    let trimmed = code.trim_start();
    if let Some(rest) = trimmed.strip_prefix("/*")
        && let Some(end) = rest.find("*/")
    {
        return rest[end + 2..].trim_start();
    }
    trimmed
}

pub(crate) fn is_declaration_block(code: &str) -> bool {
    decl_kind(skip_leading_docblock(code)).is_some()
}

struct Frame {
    class_like: bool,
    count: u32,
    doc_pending: bool,
}

enum Body {
    Empty,
    Bodyless,
    Block,
}

struct Decl<'a> {
    kind: DeclKind,
    header: &'a str,
    body: Body,
    next: usize,
}

fn is_docblock_start(line: &str) -> bool {
    line.starts_with("/*")
}

fn parse_declaration<'a>(lines: &[&'a str], i: usize) -> Option<Decl<'a>> {
    let line = lines[i];
    if let Some(head) = line.strip_suffix('{') {
        let header = head.trim_end();
        let kind = decl_kind(header)?;
        if lines.get(i + 1) == Some(&"}") {
            return Some(Decl {
                kind,
                header,
                body: Body::Empty,
                next: i + 2,
            });
        }
        return Some(Decl {
            kind,
            header,
            body: Body::Block,
            next: i + 1,
        });
    }
    if let Some(head) = line.strip_suffix(';') {
        return match decl_kind(head)? {
            DeclKind::Function => Some(Decl {
                kind: DeclKind::Function,
                header: line,
                body: Body::Bodyless,
                next: i + 1,
            }),
            DeclKind::ClassLike => None,
        };
    }
    let kind = decl_kind(line)?;
    if lines.get(i + 1) != Some(&"{") {
        return None;
    }
    if lines.get(i + 2) == Some(&"}") {
        return Some(Decl {
            kind,
            header: line,
            body: Body::Empty,
            next: i + 3,
        });
    }
    Some(Decl {
        kind,
        header: line,
        body: Body::Block,
        next: i + 2,
    })
}

fn start_doc_member(frames: &mut [Frame], out: &mut Vec<String>) {
    let Some(frame) = frames.last_mut() else {
        return;
    };
    if frame.doc_pending {
        return;
    }
    if frame.count > 0 {
        out.push(String::new());
    }
    frame.count += 1;
    frame.doc_pending = true;
}

fn account_member(frames: &mut [Frame], out: &mut Vec<String>, is_decl: bool) {
    let Some(frame) = frames.last_mut() else {
        return;
    };
    if frame.doc_pending {
        frame.doc_pending = false;
        return;
    }
    if frame.count > 0 && is_decl {
        out.push(String::new());
    }
    frame.count += 1;
}

fn adjust_frames(line: &str, frames: &mut Vec<Frame>) {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\'' || c == '"' {
            i += 1;
            while i < chars.len() && chars[i] != c {
                if chars[i] == '\\' {
                    i += 1;
                }
                i += 1;
            }
            i += 1;
            continue;
        }
        if c == '{' {
            frames.push(Frame {
                class_like: false,
                count: 0,
                doc_pending: false,
            });
        } else if c == '}' {
            frames.pop();
        }
        i += 1;
    }
}

fn emit_docblock(lines: &[&str], start: usize, out: &mut Vec<String>) -> usize {
    let mut i = start;
    while i < lines.len() {
        out.push(lines[i].to_string());
        if lines[i].contains("*/") {
            return i + 1;
        }
        i += 1;
    }
    i
}

pub(crate) fn apply_psr12_declarations(code: &str) -> String {
    let lines: Vec<&str> = code.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let mut out: Vec<String> = Vec::new();
    let mut frames: Vec<Frame> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let class_level = frames.last().is_some_and(|f| f.class_like);

        if class_level && is_docblock_start(line) {
            start_doc_member(&mut frames, &mut out);
            i = emit_docblock(&lines, i, &mut out);
            continue;
        }

        if let Some(decl) = parse_declaration(&lines, i) {
            if class_level {
                account_member(&mut frames, &mut out, true);
            }
            match decl.body {
                Body::Empty => out.push(format!("{} {{}}", decl.header)),
                Body::Bodyless => out.push(decl.header.to_string()),
                Body::Block => {
                    out.push(decl.header.to_string());
                    out.push("{".to_string());
                    frames.push(Frame {
                        class_like: decl.kind == DeclKind::ClassLike,
                        count: 0,
                        doc_pending: false,
                    });
                }
            }
            i = decl.next;
            continue;
        }

        if line == "}" {
            out.push("}".to_string());
            frames.pop();
            i += 1;
            continue;
        }

        if class_level {
            account_member(&mut frames, &mut out, false);
        }
        out.push(line.to_string());
        adjust_frames(line, &mut frames);
        i += 1;
    }
    out.join("\n")
}

#[cfg(test)]
#[path = "declaration_tests.rs"]
mod tests;
