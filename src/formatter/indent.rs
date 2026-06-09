use super::Formatter;
use super::docblock::{extract_docblock_body, merge_descriptions_and_vars};
use super::php::format_php_code;
use super::scan::{
    contains_outside_strings, count_brackets, count_leading_closers, count_unescaped_quotes, detect_heredoc,
    detect_open_quote, has_unclosed_string,
};

pub fn visual_len(s: &str) -> usize {
    s.chars().count()
}

pub fn has_switch_case(code: &str) -> bool {
    let lower = code.to_lowercase();
    (lower.contains("switch") || contains_outside_strings(&lower, "break"))
        && (lower.contains("case ") || lower.contains("default:"))
}

pub fn is_php_block_opener(code: &str) -> bool {
    let trimmed = code.trim();
    trimmed.ends_with(':') || trimmed.ends_with('{') || trimmed.contains("::begin(")
}

pub fn is_php_block_closer(code: &str) -> bool {
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

pub fn is_switch_case_peer(code: &str) -> bool {
    let lower = code.trim().to_lowercase();
    lower.starts_with("case ") || lower.starts_with("default:")
}

pub fn is_header_php_block(code: &str) -> bool {
    if code.trim_start().starts_with("/**") && !is_php_block_opener(code) {
        return true;
    }
    let has_by_line = code.lines().any(|line| {
        let t = line.trim();
        t.starts_with("use ") || t.starts_with("declare(")
    });
    if has_by_line {
        return true;
    }
    code.contains(" use ") || code.contains(";use ") || code.starts_with("use ")
}

pub fn split_header_and_opener(code: &str) -> Option<(String, String)> {
    if !is_php_block_opener(code) {
        return None;
    }
    let normalized = normalize_statements(code);
    let lines: Vec<&str> = normalized.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.len() < 2 {
        return None;
    }
    let last = lines.last()?.trim();
    let lower = last.to_lowercase();
    let is_opener = (lower.starts_with("if ")
        || lower.starts_with("if(")
        || lower.starts_with("foreach ")
        || lower.starts_with("foreach(")
        || lower.starts_with("for ")
        || lower.starts_with("for(")
        || lower.starts_with("while ")
        || lower.starts_with("while(")
        || lower.starts_with("switch "))
        && last.ends_with(':');
    if !is_opener {
        return None;
    }
    let opener = last.to_string();
    let orig_lines: Vec<&str> = code.lines().collect();
    if orig_lines.len() > 1 {
        let mut end = orig_lines.len().saturating_sub(1);
        while end > 0 && orig_lines[end].trim().is_empty() {
            end -= 1;
        }
        let mut start = 0;
        while start < end && orig_lines[start].trim().is_empty() {
            start += 1;
        }
        if start < end {
            let header = orig_lines[start..end].join("\n");
            return Some((header, opener));
        }
    }
    let header_lines: Vec<&str> = lines[..lines.len() - 1].to_vec();
    let header = header_lines.join("\n");
    Some((header, opener))
}

pub fn skip_string_literal(chars: &[char], start: usize, result: &mut String) -> usize {
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

fn expand_inline_docblock(comment: &str) -> String {
    let trimmed = comment.trim();
    if !trimmed.starts_with("/**") || !trimmed.ends_with("*/") || trimmed.len() < 5 {
        return comment.to_string();
    }
    let inner = &trimmed[3..trimmed.len() - 2].trim();

    if !inner.contains("* @") {
        return comment.to_string();
    }

    let mut bodies: Vec<&str> = Vec::new();
    let mut rest = *inner;
    while let Some(pos) = rest.find("* @") {
        let before = rest[..pos].trim().trim_end_matches('*').trim();
        if !before.is_empty() {
            bodies.push(before);
        }
        rest = &rest[pos + 2..];
    }
    let last = rest.trim().trim_end_matches('*').trim();
    if !last.is_empty() {
        bodies.push(last);
    }

    if bodies.len() <= 1 {
        return comment.to_string();
    }

    let mut result = String::from("/**\n");
    for body in &bodies {
        result.push_str(&format!(" * {body}\n"));
    }
    result.push_str(" */");
    result
}

fn emit_block_comment_break(result: &mut String) {
    if !result.ends_with('\n') && result.trim_end().len() > 1 {
        let last = result.pop().unwrap_or_default();
        if last != ' ' && last != '\n' {
            result.push(last);
        }
        if !result.ends_with('\n') {
            result.push('\n');
        }
    }
}

fn collect_block_comment(chars: &[char], start: usize) -> (String, usize) {
    let mut comment = String::from("/*");
    let mut i = start + 2;
    let len = chars.len();
    while i < len {
        comment.push(chars[i]);
        if chars[i] == '*' && i + 1 < len && chars[i + 1] == '/' {
            comment.push(chars[i + 1]);
            i += 2;
            return (comment, i);
        }
        i += 1;
    }
    (comment, i)
}

fn push_brace_breaks(ch: char, brace_depth: i32, next_char: Option<char>, result: &mut String) -> bool {
    if ch == '{' && brace_depth > 0 && next_char.is_some_and(|c| c != '\n') {
        result.push(ch);
        result.push('\n');
        return true;
    }
    if ch == '}' && brace_depth >= 0 && !result.ends_with('\n') {
        result.push('\n');
    }
    false
}

fn process_block_comment(chars: &[char], start: usize, result: &mut String) -> usize {
    emit_block_comment_break(result);
    let (comment, i) = collect_block_comment(chars, start);
    result.push_str(&expand_inline_docblock(&comment));
    if !result.ends_with('\n') {
        result.push('\n');
    }
    if i < chars.len() && chars[i] != '\n' {
        result.push('\n');
    }
    i
}

pub fn normalize_statements(code: &str) -> String {
    let mut result = String::from("\n");
    let chars: Vec<char> = code.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut paren_depth: i32 = 0;
    let mut brace_depth: i32 = 0;
    let mut bracket_depth: i32 = 0;
    let mut in_case_label = false;

    while i < len {
        let ch = chars[i];
        if ch == '\'' || ch == '"' {
            i = skip_string_literal(&chars, i, &mut result);
            continue;
        }
        if ch == '/' && i + 1 < len && chars[i + 1] == '*' {
            i = process_block_comment(&chars, i, &mut result);
            continue;
        }
        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth -= 1,
            '{' => brace_depth += 1,
            '}' => brace_depth -= 1,
            '[' => bracket_depth += 1,
            ']' => bracket_depth -= 1,
            _ => {}
        }
        let next = if i + 1 < len { Some(chars[i + 1]) } else { None };
        if push_brace_breaks(ch, brace_depth, next, &mut result) {
            i += 1;
            continue;
        }
        if paren_depth <= 0 && (matches_keyword_at(&chars, i, "case ") || matches_keyword_at(&chars, i, "default:")) {
            if !result.ends_with('\n') && !result.trim_end().is_empty() {
                result.push('\n');
            }
            in_case_label = true;
        }
        if in_case_label && ch == ':' && paren_depth <= 0 && bracket_depth <= 0 {
            if next == Some(':') {
                result.push_str("::");
                i += 2;
                continue;
            }
            result.push_str(":\n");
            in_case_label = false;
            i += 1;
            continue;
        }
        result.push(ch);
        let next_is_nl = next.is_some_and(|c| c == '\n');
        if ch == ';' && paren_depth <= 0 && !next_is_nl {
            result.push('\n');
        }
        if ch == ',' && brace_depth > 0 && paren_depth <= 0 && bracket_depth <= 0 && !next_is_nl {
            result.push('\n');
        }
        i += 1;
    }
    result
}

pub fn matches_keyword_at(chars: &[char], pos: usize, keyword: &str) -> bool {
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

impl Formatter {
    pub(crate) fn emit_reindented_line(&self, formatted: &str, pad: &str, depth: &mut i32, result: &mut String) {
        let leading = (count_leading_closers(formatted) as i32).min(1);
        let is_continuation = formatted.starts_with("? ")
            || formatted.starts_with(": ")
            || formatted.starts_with("|| ")
            || formatted.starts_with("&& ")
            || formatted.starts_with(". ")
            || formatted.starts_with("->");
        let extra = i32::from(is_continuation);
        let write_depth = (*depth - leading + extra).max(0) as usize;
        let inner_pad = self.indent.repeat(write_depth);
        let base_pad = format!("{pad}{inner_pad}");
        if let Some((head, tail)) = split_trailing_array_item_close(formatted) {
            result.push_str(&format!("{base_pad}{head}\n"));
            let close_depth = write_depth.saturating_sub(1);
            let close_pad = format!("{pad}{}", self.indent.repeat(close_depth));
            result.push_str(&format!("{close_pad}{tail}\n"));
        } else if let Some(split) = self.try_split_long_line(formatted, &base_pad) {
            result.push_str(&split);
        } else if formatted.starts_with('*') {
            result.push_str(&format!("{pad}{inner_pad} {formatted}\n"));
        } else {
            result.push_str(&format!("{pad}{inner_pad}{formatted}\n"));
        }
        let (openers, closers) = count_brackets(formatted);
        let net = openers as i32 - closers as i32;
        *depth += net.clamp(-1, 1);
        *depth = (*depth).max(0);
    }
}

fn split_trailing_array_item_close(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    let (core, suffix) = if let Some(no_comma) = trimmed.strip_suffix(',') {
        (no_comma.trim_end(), ",")
    } else if let Some(no_semicolon) = trimmed.strip_suffix(';') {
        (no_semicolon.trim_end(), ";")
    } else {
        return None;
    };

    if !core.ends_with(']') {
        return None;
    }

    let (openers, closers) = count_brackets(core);
    if closers != openers + 1 {
        return None;
    }

    let head = core.strip_suffix(']')?.trim_end();
    if head.is_empty() {
        return None;
    }

    let mut first_line = head.to_string();
    if !first_line.ends_with(',') {
        first_line.push(',');
    }

    Some((first_line, format!("]{suffix}")))
}

pub fn join_ternary_lines(code: &str) -> String {
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
        let mut saw_false_branch = false;
        let mut false_branch_depth: i32 = 0;
        i += 1;
        while i < lines.len() {
            let next = lines[i].trim();
            if next.is_empty() {
                break;
            }
            joined.push(' ');
            joined.push_str(next);

            if let Some(false_part) = next.strip_prefix(':').map(str::trim_start) {
                saw_false_branch = true;
                false_branch_depth = 0;
                let (o, c) = count_brackets(false_part);
                false_branch_depth += o as i32 - c as i32;
                if false_branch_depth <= 0 && (false_part.ends_with(',') || false_part.ends_with(';')) {
                    i += 1;
                    break;
                }
            } else if saw_false_branch {
                let (o, c) = count_brackets(next);
                false_branch_depth += o as i32 - c as i32;
                if false_branch_depth <= 0 && (next.ends_with(',') || next.ends_with(';')) {
                    i += 1;
                    break;
                }
            }

            i += 1;
            if next.contains(';') {
                break;
            }
        }
        result.push(joined);
    }

    result.join("\n")
}

impl Formatter {
    pub(crate) fn reindent_php_block(&self, code: &str, pad: &str) -> String {
        let needs_normalize = !code.contains('\n') && (code.contains(';') || has_switch_case(code));
        let code = if needs_normalize {
            normalize_statements(code)
        } else {
            join_ternary_lines(code)
        };
        let is_header = is_header_php_block(&code);
        Reindenter::new(self, pad, is_header).run(&code)
    }
}

struct Reindenter<'a> {
    fmt: &'a Formatter,
    pad: &'a str,
    is_header: bool,
    result: String,
    depth: i32,
    switch_levels: Vec<(i32, bool)>,
    prev_blank: bool,
    first_content: bool,
    prev_was_doc_close: bool,
    prev_was_declare: bool,
    in_string: Option<char>,
    heredoc_marker: Option<String>,
    pending_docblocks: Vec<String>,
    pending_descriptions: Vec<String>,
    deferred_lines: Vec<String>,
    in_docblock: bool,
    docblock_bodies: Vec<String>,
}

impl<'a> Reindenter<'a> {
    fn new(fmt: &'a Formatter, pad: &'a str, is_header: bool) -> Self {
        Self {
            fmt,
            pad,
            is_header,
            result: String::new(),
            depth: 0,
            switch_levels: Vec::new(),
            prev_blank: false,
            first_content: true,
            prev_was_doc_close: false,
            prev_was_declare: false,
            in_string: None,
            heredoc_marker: None,
            pending_docblocks: Vec::new(),
            pending_descriptions: Vec::new(),
            deferred_lines: Vec::new(),
            in_docblock: false,
            docblock_bodies: Vec::new(),
        }
    }

    fn run(mut self, code: &str) -> String {
        for line in code.lines() {
            self.process_line(line);
        }
        self.finish()
    }

    fn process_line(&mut self, line: &str) {
        if self.heredoc_marker.is_some() {
            self.continue_heredoc(line);
            return;
        }
        if self.in_string.is_some() {
            self.continue_string(line);
            return;
        }
        let trimmed = line.trim();
        if self.in_docblock {
            self.consume_docblock(trimmed);
            return;
        }
        if trimmed.is_empty() {
            self.handle_blank();
            return;
        }
        if self.first_content && !self.prev_blank && self.is_header {
            self.result.push('\n');
        }
        self.first_content = false;
        let is_use_import = trimmed.starts_with("use ");
        let is_declare = trimmed.starts_with("declare(");
        if self.absorb_pending(trimmed, is_use_import, is_declare) {
            return;
        }
        if self.prev_was_declare && !is_declare && !self.prev_blank {
            self.result.push('\n');
        }
        if self.prev_was_doc_close && !self.prev_blank {
            self.result.push('\n');
        }
        self.prev_blank = false;
        self.prev_was_doc_close = trimmed == "*/";
        self.prev_was_declare = is_declare;
        if let Some(body) = extract_docblock_body(trimmed) {
            self.pending_docblocks.push(body);
            self.prev_was_declare = false;
            return;
        }
        if trimmed == "/**" {
            self.in_docblock = true;
            self.docblock_bodies.clear();
            return;
        }
        self.emit_code(trimmed);
    }

    fn continue_heredoc(&mut self, line: &str) {
        self.result.push_str(line);
        self.result.push('\n');
        let Some(marker) = self.heredoc_marker.clone() else {
            return;
        };
        let closing = line.trim().trim_end_matches(';');
        if closing == marker {
            self.heredoc_marker = None;
            let after_marker = line.trim().strip_prefix(marker.as_str()).unwrap_or("");
            let (o, c) = count_brackets(after_marker);
            self.depth = (self.depth + o as i32 - c as i32).max(0);
        }
    }

    fn continue_string(&mut self, line: &str) {
        self.result.push_str(line);
        self.result.push('\n');
        let Some(quote) = self.in_string else {
            return;
        };
        if count_unescaped_quotes(line, quote) % 2 == 1 {
            self.in_string = None;
            if let Some(pos) = line.rfind(quote) {
                let (o, c) = count_brackets(&line[pos + 1..]);
                self.depth = (self.depth + o as i32 - c as i32).max(0);
            }
        }
    }

    fn consume_docblock(&mut self, trimmed: &str) {
        if trimmed == "*/" || trimmed == "**/" {
            self.in_docblock = false;
            self.close_docblock();
        } else if let Some(body) = trimmed.strip_prefix("* ") {
            self.docblock_bodies.push(body.to_string());
        } else if trimmed == "*" {
            self.docblock_bodies.push(String::new());
        }
    }

    fn close_docblock(&mut self) {
        let all_var = !self.docblock_bodies.is_empty() && self.docblock_bodies.iter().all(|b| b.starts_with("@var "));
        if all_var {
            self.pending_docblocks.append(&mut self.docblock_bodies);
        } else if self.is_header {
            self.pending_descriptions.append(&mut self.docblock_bodies);
        } else {
            self.flush_pending_docblocks();
            let fmt = self.fmt;
            fmt.emit_reindented_line("/**", self.pad, &mut self.depth, &mut self.result);
            let bodies = std::mem::take(&mut self.docblock_bodies);
            for body in &bodies {
                fmt.emit_reindented_line(&format!("* {body}"), self.pad, &mut self.depth, &mut self.result);
            }
            fmt.emit_reindented_line("*/", self.pad, &mut self.depth, &mut self.result);
            self.prev_was_doc_close = true;
        }
    }

    fn handle_blank(&mut self) {
        if !self.prev_blank && !self.first_content {
            if self.pending_docblocks.is_empty() && !self.in_docblock {
                self.result.push('\n');
            }
            self.prev_blank = true;
        }
    }

    fn absorb_pending(&mut self, trimmed: &str, is_use_import: bool, is_declare: bool) -> bool {
        let has_pending = !self.pending_docblocks.is_empty() || !self.pending_descriptions.is_empty();
        if !has_pending {
            return false;
        }
        if extract_docblock_body(trimmed).is_none() && trimmed != "/**" && !is_use_import && !is_declare {
            if !self.deferred_lines.is_empty() {
                self.flush_deferred();
                self.result.push('\n');
            }
            self.flush_pending_docblocks();
            self.prev_was_doc_close = true;
            self.prev_blank = false;
            false
        } else if is_use_import || is_declare {
            self.deferred_lines.push(trimmed.to_string());
            self.prev_was_declare = is_declare;
            true
        } else {
            false
        }
    }

    fn flush_pending_docblocks(&mut self) {
        if self.pending_docblocks.is_empty() && self.pending_descriptions.is_empty() {
            return;
        }
        let merged = merge_descriptions_and_vars(&self.pending_descriptions, &self.pending_docblocks);
        self.fmt
            .flush_docblocks(&merged, self.pad, &mut self.depth, &mut self.result);
        self.pending_docblocks.clear();
        self.pending_descriptions.clear();
    }

    fn flush_deferred(&mut self) {
        if self.deferred_lines.is_empty() {
            return;
        }
        emit_deferred_lines(
            self.fmt,
            &self.deferred_lines,
            self.pad,
            &mut self.depth,
            &mut self.result,
        );
        self.deferred_lines.clear();
    }

    fn emit_code(&mut self, trimmed: &str) {
        let formatted = format_php_code(trimmed);
        let lower_fmt = formatted.to_lowercase();
        let is_switch_opener = lower_fmt.starts_with("switch") && formatted.ends_with('{');
        let is_case_label = lower_fmt.starts_with("case ") || lower_fmt.starts_with("default:");
        let closes_block = formatted.starts_with('}');
        let case_extra = self.case_extra(is_case_label, closes_block);
        let fmt = self.fmt;
        if case_extra > 0 {
            let case_pad = format!("{}{}", self.pad, fmt.indent.repeat(case_extra));
            fmt.emit_reindented_line(&formatted, &case_pad, &mut self.depth, &mut self.result);
        } else {
            fmt.emit_reindented_line(&formatted, self.pad, &mut self.depth, &mut self.result);
        }
        self.update_switch_levels(is_switch_opener, is_case_label, closes_block);
        if let Some(marker) = detect_heredoc(trimmed) {
            self.heredoc_marker = Some(marker);
        } else if has_unclosed_string(trimmed) {
            self.in_string = detect_open_quote(trimmed);
        }
    }

    fn case_extra(&self, is_case_label: bool, closes_block: bool) -> usize {
        match self.switch_levels.last() {
            Some(&(label_depth, in_body)) => {
                let closes_switch = closes_block && self.depth <= label_depth;
                let label_here = is_case_label && self.depth == label_depth;
                usize::from(in_body && !closes_switch && !label_here)
            }
            None => 0,
        }
    }

    fn update_switch_levels(&mut self, is_switch_opener: bool, is_case_label: bool, closes_block: bool) {
        if is_switch_opener {
            self.switch_levels.push((self.depth, false));
        } else if is_case_label {
            if let Some(entry) = self.switch_levels.last_mut()
                && self.depth == entry.0
            {
                entry.1 = true;
            }
        } else if closes_block
            && let Some(&(label_depth, _)) = self.switch_levels.last()
            && self.depth < label_depth
        {
            self.switch_levels.pop();
        }
    }

    fn finish(mut self) -> String {
        if !self.pending_docblocks.is_empty() || !self.pending_descriptions.is_empty() {
            if !self.deferred_lines.is_empty() {
                self.flush_deferred();
                self.result.push('\n');
            }
            self.flush_pending_docblocks();
        } else if !self.deferred_lines.is_empty() {
            self.flush_deferred();
        }
        let result = self.result.trim_end_matches('\n').to_string() + "\n";
        sort_use_lines(&result)
    }
}

fn sort_use_lines(code: &str) -> String {
    let lines: Vec<&str> = code.lines().collect();
    let mut result: Vec<String> = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.starts_with("use ") && trimmed.ends_with(';') {
            let mut use_group: Vec<&str> = Vec::new();
            while i < lines.len() && lines[i].trim().starts_with("use ") && lines[i].trim().ends_with(';') {
                use_group.push(lines[i]);
                i += 1;
            }
            use_group.sort_by_key(|a| a.trim().to_lowercase());
            use_group.dedup_by(|a, b| a.trim() == b.trim());

            for line in use_group {
                result.push(line.to_string());
            }
            if i < lines.len() && !lines[i].trim().is_empty() {
                result.push(String::new());
            }
        } else {
            result.push(lines[i].to_string());
            i += 1;
        }
    }

    result.join("\n") + "\n"
}

fn emit_deferred_lines(formatter: &Formatter, deferred: &[String], pad: &str, depth: &mut i32, result: &mut String) {
    let mut declares = Vec::new();
    let mut others = Vec::new();
    for dl in deferred {
        if dl.trim().starts_with("declare(") {
            declares.push(dl.clone());
        } else {
            others.push(dl.clone());
        }
    }
    for dl in &declares {
        formatter.emit_reindented_line(dl, pad, depth, result);
    }
    if !declares.is_empty() && !others.is_empty() {
        result.push('\n');
    }
    for dl in &others {
        formatter.emit_reindented_line(dl, pad, depth, result);
    }
}
