use super::Formatter;
use super::docblock::{emit_docblock_php, expand_single_line_docblock, is_docblock_only};
use super::echo::{contains_break, is_echo_block_closer, is_echo_block_opener};
use super::indent::{
    has_switch_case, is_header_php_block, is_php_block_closer, is_php_block_opener, is_switch_case_peer,
    split_header_and_opener, visual_len,
};
use super::normalize::normalize_statements;
use super::php::format_php_code;
use super::scan::{
    count_semicolons_outside_parens, count_top_level_semicolons, find_ternary_positions, has_expandable_closure,
};

#[derive(Clone)]
pub(crate) struct PhpDepthState {
    pub(crate) depth: usize,
    pub(crate) switch_stack: Vec<usize>,
}

fn is_self_contained_brace_switch(code: &str) -> bool {
    let trimmed = code.trim();
    if !trimmed.to_lowercase().starts_with("switch") || !trimmed.ends_with('}') {
        return false;
    }
    let chars: Vec<char> = trimmed.chars().collect();
    let mut braces = 0i32;
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '\'' || ch == '"' {
            i += 1;
            while i < chars.len() && chars[i] != ch {
                if chars[i] == '\\' {
                    i += 1;
                }
                i += 1;
            }
        } else if ch == '{' {
            braces += 1;
        } else if ch == '}' {
            braces -= 1;
        }
        i += 1;
    }
    braces == 0
}

impl Formatter {
    fn emit_switch_stmt(&self, trimmed: &str, state: &mut PhpDepthState, output: &mut String) {
        let indent = &self.indent;
        let formatted = format_php_code(trimmed);
        let lower = trimmed.to_lowercase();
        if lower.starts_with("switch") && is_php_block_opener(trimmed) {
            let pad = indent.repeat(state.depth);
            output.push_str(&format!("{pad}<?php {formatted} ?>\n"));
            state.switch_stack.push(state.depth);
            state.depth += 1;
        } else if is_switch_case_peer(trimmed) {
            let lvl = state
                .switch_stack
                .last()
                .copied()
                .unwrap_or(state.depth.saturating_sub(1));
            let case_lvl = lvl + 1;
            let pad = indent.repeat(case_lvl);
            output.push_str(&format!("{pad}<?php {formatted} ?>\n"));
            state.depth = case_lvl + 1;
        } else if lower.starts_with("endswitch") || (trimmed == "}" && !state.switch_stack.is_empty()) {
            let lvl = state.switch_stack.pop().unwrap_or(state.depth.saturating_sub(1));
            let pad = indent.repeat(lvl);
            output.push_str(&format!("{pad}<?php {formatted} ?>\n"));
            state.depth = lvl;
        } else if contains_break(&lower) {
            let pad = indent.repeat(state.depth);
            output.push_str(&format!("{pad}<?php {formatted} ?>\n"));
        } else {
            let pad = indent.repeat(state.depth);
            output.push_str(&format!("{pad}<?php {formatted} ?>\n"));
            if is_php_block_opener(trimmed) {
                state.depth += 1;
            }
        }
    }

    fn emit_multiline_php(&self, code: &str, pad: &str, depth: &mut usize, output: &mut String) {
        let is_header = is_header_php_block(code);
        if is_header {
            if let Some((header_code, opener_line)) = split_header_and_opener(code) {
                output.push_str(&format!("{pad}<?php\n"));
                let reindented = self.reindent_php_block(&header_code, pad);
                output.push_str(&reindented);
                output.push('\n');
                output.push_str(&format!("{pad}?>\n"));
                let formatted = format_php_code(&opener_line);
                output.push_str(&format!("{pad}<?php {formatted} ?>\n"));
                *depth += 1;
                return;
            }
            output.push_str(&format!("{pad}<?php\n"));
            let reindented = self.reindent_php_block(code, pad);
            output.push_str(&reindented);
            output.push('\n');
            output.push_str(&format!("{pad}?>\n"));
        } else {
            self.emit_multiline_php_inline(code, pad, output);
        }
        let has_widget_begin = code.contains("::begin(");
        let has_widget_end = code.contains("::end(");
        if has_widget_begin || has_widget_end || !is_header {
            if has_widget_end {
                *depth = depth.saturating_sub(1);
            }
            if has_widget_begin || (!is_php_block_closer(code) && is_php_block_opener(code)) {
                *depth += 1;
            }
        }
    }

    fn emit_multiline_php_inline(&self, code: &str, pad: &str, output: &mut String) {
        let reindented = self.reindent_php_block(code, pad);
        let lines: Vec<&str> = reindented.lines().filter(|l| !l.trim().is_empty()).collect();
        if lines.len() > 1 {
            if lines[0].trim_start().starts_with("/**") {
                output.push_str(&format!("{pad}<?php\n"));
                for line in &lines[..lines.len() - 1] {
                    output.push_str(line);
                    output.push('\n');
                }
            } else {
                output.push_str(&format!("{pad}<?php {}\n", lines[0].trim_start()));
                for line in &lines[1..lines.len() - 1] {
                    output.push_str(line);
                    output.push('\n');
                }
            }
            output.push_str(&format!("{} ?>\n", lines[lines.len() - 1]));
        } else if lines.len() == 1 {
            let formatted = lines[0].trim_start();
            if let Some(split) = self.split_inline_php(formatted, pad) {
                let split_lines: Vec<&str> = split.lines().filter(|l| !l.trim().is_empty()).collect();
                self.emit_php_lines(&split_lines, pad, output);
            } else {
                output.push_str(&format!("{pad}<?php {formatted} ?>\n"));
            }
        }
    }

    fn emit_single_php(&self, code: &str, pad: &str, state: &mut PhpDepthState, output: &mut String) {
        let indent = &self.indent;
        let formatted = format_php_code(code);
        let lower = code.trim().to_lowercase();
        if lower.starts_with("switch") && is_php_block_opener(code) && !is_php_block_closer(code) {
            let stmt_pad = indent.repeat(state.depth);
            output.push_str(&format!("{stmt_pad}<?php {formatted} ?>\n"));
            state.switch_stack.push(state.depth);
            state.depth += 1;
        } else if !state.switch_stack.is_empty() && is_switch_case_peer(code) {
            let lvl = state
                .switch_stack
                .last()
                .copied()
                .unwrap_or(state.depth.saturating_sub(1));
            let case_lvl = lvl + 1;
            let stmt_pad = indent.repeat(case_lvl);
            output.push_str(&format!("{stmt_pad}<?php {formatted} ?>\n"));
            state.depth = case_lvl + 1;
        } else if !state.switch_stack.is_empty() && lower.starts_with("endswitch") {
            let lvl = state.switch_stack.pop().unwrap_or(state.depth.saturating_sub(1));
            let stmt_pad = indent.repeat(lvl);
            output.push_str(&format!("{stmt_pad}<?php {formatted} ?>\n"));
            state.depth = lvl;
        } else if !state.switch_stack.is_empty() && contains_break(&lower) {
            let stmt_pad = indent.repeat(state.depth);
            output.push_str(&format!("{stmt_pad}<?php {formatted} ?>\n"));
        } else if !state.switch_stack.is_empty() && code.trim() == "}" {
            let lvl = state.switch_stack.pop().unwrap_or(state.depth.saturating_sub(1));
            let stmt_pad = indent.repeat(lvl);
            output.push_str(&format!("{stmt_pad}<?php {formatted} ?>\n"));
            state.depth = lvl;
        } else if is_php_block_closer(code) {
            state.depth = state.depth.saturating_sub(1);
            let pad_less = indent.repeat(state.depth);
            output.push_str(&format!("{pad_less}<?php {formatted} ?>\n"));
            if is_php_block_opener(code) {
                state.depth += 1;
            }
        } else {
            self.emit_single_php_long(code, pad, &mut state.depth, output);
        }
    }

    fn emit_single_php_long(&self, code: &str, pad: &str, depth: &mut usize, output: &mut String) {
        let formatted = format_php_code(code);
        if is_header_php_block(code) {
            self.emit_php_header_block(code, pad, output);
            return;
        }
        if let Some(docblock) = expand_single_line_docblock(code) {
            emit_docblock_php(&docblock, pad, output);
            return;
        }
        let single = format!("{pad}<?php {formatted} ?>");
        let is_alt_syntax_opener = code.trim().ends_with(':');
        if (visual_len(&single) <= self.max_line_length && !has_expandable_closure(&formatted)) || is_alt_syntax_opener
        {
            output.push_str(&format!("{single}\n"));
            if is_php_block_opener(code) {
                *depth += 1;
            }
            return;
        }
        if let Some((q_pos, c_pos)) = find_ternary_positions(&formatted) {
            let condition = formatted[..q_pos].trim_end();
            let true_val = formatted[q_pos + 1..c_pos].trim();
            let false_val = formatted[c_pos + 1..].trim();
            let inner_pad = format!("{pad}{}", self.indent);
            output.push_str(&format!(
                "{pad}<?php {condition}\n{inner_pad}? {true_val}\n{inner_pad}: {false_val} ?>\n"
            ));
        } else if let Some(split) = self
            .split_inline_php(&formatted, pad)
            .or_else(|| self.expand_braced_value(&formatted, pad))
        {
            let lines: Vec<&str> = split.lines().filter(|l| !l.trim().is_empty()).collect();
            self.emit_php_lines(&lines, pad, output);
        } else {
            self.emit_reindented_php_block(code, pad, output);
        }
        if is_php_block_opener(code) {
            *depth += 1;
        }
    }

    fn emit_php_header_block(&self, code: &str, pad: &str, output: &mut String) {
        output.push_str(&format!("{pad}<?php\n"));
        output.push_str(&self.reindent_php_block(code, pad));
        output.push('\n');
        output.push_str(&format!("{pad}?>\n"));
    }

    fn emit_php_lines(&self, lines: &[&str], pad: &str, output: &mut String) {
        if lines.len() > 1 {
            output.push_str(&format!("{pad}<?php {}\n", lines[0].trim_start()));
            for line in &lines[1..lines.len() - 1] {
                output.push_str(line);
                output.push('\n');
            }
            output.push_str(&format!("{} ?>\n", lines[lines.len() - 1]));
        } else if let Some(one) = lines.first() {
            output.push_str(&format!("{pad}<?php {} ?>\n", one.trim_start()));
        }
    }

    fn emit_reindented_php_block(&self, code: &str, pad: &str, output: &mut String) {
        let reindented = self.reindent_php_block(code, pad);
        let lines: Vec<&str> = reindented.lines().filter(|l| !l.trim().is_empty()).collect();
        if lines.len() > 1 {
            self.emit_php_lines(&lines, pad, output);
        } else {
            output.push_str(&format!("{pad}<?php\n"));
            output.push_str(&reindented);
            output.push_str(&format!("{pad}?>\n"));
        }
    }

    pub(crate) fn emit_php_block(&self, code: &str, pad: &str, state: &mut PhpDepthState, output: &mut String) {
        let trimmed = code.trim();
        if let Some(expr) = trimmed.strip_prefix("echo ") {
            let expr = expr.strip_suffix(';').unwrap_or(expr).trim();
            let semicolons = count_semicolons_outside_parens(code);
            if semicolons <= 1 && !expr.contains('\n') {
                self.emit_php_echo(expr, pad, state, output);
                return;
            }
        }
        if is_docblock_only(code) {
            emit_docblock_php(code, pad, output);
            return;
        }
        let semicolons = count_top_level_semicolons(code);
        let is_multiline = code.contains('\n') || semicolons > 1 || has_switch_case(code);
        if is_multiline && has_switch_case(code) && !is_self_contained_brace_switch(code) {
            self.emit_php_switch_block(code, state, output);
        } else if is_multiline {
            self.emit_multiline_php(code, pad, &mut state.depth, output);
        } else {
            self.emit_single_php(code, pad, state, output);
        }
    }

    fn emit_php_switch_block(&self, code: &str, state: &mut PhpDepthState, output: &mut String) {
        let indent = &self.indent;
        let normalized = normalize_statements(code);
        let statements: Vec<&str> = normalized
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect();
        let mut i = 0usize;
        while i < statements.len() {
            let current = statements[i];
            if current.to_lowercase().starts_with("switch")
                && is_php_block_opener(current)
                && i + 1 < statements.len()
                && is_switch_case_peer(statements[i + 1])
            {
                let switch_depth = state.depth;
                let stmt_pad = indent.repeat(switch_depth);
                let switch_stmt = format_php_code(current);
                let first_case = format_php_code(statements[i + 1]);
                let case_pad = format!("{stmt_pad}{indent}");
                output.push_str(&format!("{stmt_pad}<?php {switch_stmt}\n{case_pad}{first_case} ?>\n"));
                state.switch_stack.push(switch_depth);
                state.depth = switch_depth + 2;
                i += 2;
                continue;
            }
            self.emit_switch_stmt(current, state, output);
            i += 1;
        }
    }

    pub(crate) fn emit_php_echo(&self, code: &str, pad: &str, state: &mut PhpDepthState, output: &mut String) {
        if is_echo_block_closer(code) {
            state.depth = state.depth.saturating_sub(1);
            let pad = self.indent.repeat(state.depth);
            output.push_str(&self.format_echo(code, &pad));
        } else {
            output.push_str(&self.format_echo(code, pad));
            if is_echo_block_opener(code) {
                state.depth += 1;
            }
        }
    }
}
