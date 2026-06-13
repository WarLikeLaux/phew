use super::Formatter;
use super::indent::visual_len;
use super::php::split_by_args;
use super::scan::{
    BinaryOp, array_is_list, bracket_balance, count_leading_closers, count_top_level_semicolons, find_array_arrow,
    find_brace_block, find_closure_body, find_matching_close, find_ternary_positions, find_top_level_assignment_equal,
    find_top_level_binary_op, find_top_level_fat_arrow, has_expandable_closure, normalize_closure_body,
    split_by_commas, split_by_commas_with_depth,
};

const PHP_INLINE_TAG_WIDTH: usize = "<?php ".len() + " ?>".len();

fn control_condition_prefix(prefix: &str) -> Option<&str> {
    matches!(prefix, "if" | "elseif" | "while").then_some(prefix)
}

fn logical_operands(formatted: &str, op: BinaryOp, positions: &[usize]) -> Option<Vec<String>> {
    let chars: Vec<char> = formatted.chars().collect();
    let mut operands: Vec<String> = Vec::with_capacity(positions.len() + 1);
    let mut start = 0;
    for &pos in positions {
        operands.push(chars[start..pos].iter().collect::<String>().trim().to_string());
        start = pos + op.char_len();
    }
    operands.push(chars[start..].iter().collect::<String>().trim().to_string());

    if operands.iter().any(String::is_empty) || !logical_roundtrip_ok(formatted, &operands, op.token()) {
        return None;
    }
    Some(operands)
}

impl Formatter {
    pub(crate) fn append_ternary_value(&self, result: &mut String, marker: char, value: &str, line_pad: &str) {
        let single_len = visual_len(line_pad) + 2 + visual_len(value);
        if single_len <= self.max_line_length {
            result.push_str(&format!("{line_pad}{marker} {value}\n"));
            return;
        }

        if let Some(split) = self.try_split_long_line(value, line_pad) {
            let mut lines = split.lines();
            if let Some(first) = lines.next() {
                let first = first.strip_prefix(line_pad).unwrap_or(first).trim_start();
                result.push_str(&format!("{line_pad}{marker} {first}\n"));
                for line in lines {
                    if !line.trim().is_empty() {
                        result.push_str(line);
                        result.push('\n');
                    }
                }
                return;
            }
        }

        result.push_str(&format!("{line_pad}{marker} {value}\n"));
    }

    pub(crate) fn try_split_long_line(&self, formatted: &str, base_pad: &str) -> Option<String> {
        if visual_len(base_pad) + visual_len(formatted) <= self.max_line_length && !has_expandable_closure(formatted) {
            return None;
        }
        self.split_long_line(formatted, base_pad)
    }

    pub(crate) fn split_inline_php(&self, formatted: &str, base_pad: &str) -> Option<String> {
        let inline_width = visual_len(base_pad) + PHP_INLINE_TAG_WIDTH + visual_len(formatted);
        if inline_width <= self.max_line_length && !has_expandable_closure(formatted) {
            return None;
        }
        self.split_long_line(formatted, base_pad)
    }

    pub(crate) fn split_long_line(&self, formatted: &str, base_pad: &str) -> Option<String> {
        if let Some(split) = self.split_control_condition(formatted, base_pad) {
            return Some(split);
        }

        if let Some(split) = self.split_long_by_commas(formatted, base_pad) {
            return Some(split);
        }

        if let Some((q_pos, c_pos)) = find_ternary_positions(formatted) {
            let condition = formatted[..q_pos].trim_end();
            let true_val = formatted[q_pos + 1..c_pos].trim();
            let false_val = formatted[c_pos + 1..].trim();
            let inner_pad = format!("{base_pad}{}", self.indent);
            let mut result = format!("{base_pad}{condition}\n");
            self.append_ternary_value(&mut result, '?', true_val, &inner_pad);
            self.append_ternary_value(&mut result, ':', false_val, &inner_pad);
            return Some(result);
        }

        if let Some(split) = self.split_parenthesized_expression(formatted, base_pad) {
            return Some(split);
        }

        if let Some(split) = self.split_long_by_logical(formatted, base_pad) {
            return Some(split);
        }

        if let Some((prefix, args, suffix)) = split_by_args(formatted) {
            return Some(self.build_split(&prefix, &args, &suffix, base_pad));
        }

        if let Some(expanded) = self.expand_assignment_array(formatted, base_pad) {
            return Some(expanded);
        }

        if let Some(expanded) = self.expand_bare_array_with_suffix(formatted, base_pad) {
            return Some(expanded);
        }

        let chars: Vec<char> = formatted.chars().collect();
        if let Some(open_pos) = chars.iter().position(|&c| c == '(')
            && let Some(close_pos) = find_matching_close(&chars, open_pos)
        {
            let inner: String = chars[open_pos + 1..close_pos].iter().collect();
            let inner = inner.trim();

            if let Some(array_inner) = inner.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                let items = split_by_commas(array_inner);
                let single_too_long = items.len() == 1
                    && visual_len(base_pad) + visual_len(&self.indent) + visual_len(&items[0]) + 1
                        > self.max_line_length;
                if items.len() > 1 || single_too_long || items.iter().any(|it| has_expandable_closure(it)) {
                    let prefix: String = chars[..=open_pos].iter().collect();
                    let suffix: String = chars[close_pos..].iter().collect();
                    let new_prefix = format!("{prefix}[");
                    let new_suffix = format!("]{suffix}");
                    return Some(self.build_split(&new_prefix, &items, &new_suffix, base_pad));
                }
            }
        }

        if let Some(expanded) = self.expand_nested_array(formatted, base_pad) {
            return Some(expanded);
        }

        if let Some(expanded) = self.expand_braced_value(formatted, base_pad) {
            return Some(expanded);
        }

        if formatted.trim_end().ends_with(';')
            && let Some(expanded) = self.expand_inline_closure(formatted, base_pad)
        {
            return Some(expanded);
        }

        None
    }

    fn split_control_condition(&self, formatted: &str, base_pad: &str) -> Option<String> {
        let chars: Vec<char> = formatted.chars().collect();
        let open_pos = chars.iter().position(|&c| c == '(')?;
        let close_pos = find_matching_close(&chars, open_pos)?;
        let prefix: String = chars[..open_pos].iter().collect();
        let prefix = control_condition_prefix(prefix.trim_end())?;
        let suffix: String = chars[close_pos + 1..].iter().collect();
        let suffix = suffix.trim();
        if suffix != ":" && suffix != "{" {
            return None;
        }
        let inner: String = chars[open_pos + 1..close_pos].iter().collect();
        let inner = inner.trim();
        let inner_pad = format!("{base_pad}{}", self.indent);
        let split_inner = self
            .split_condition_logical(inner, &inner_pad)
            .or_else(|| self.split_long_line(inner, &inner_pad))
            .unwrap_or_else(|| format!("{inner_pad}{inner}\n"));
        let mut result = format!("{base_pad}{prefix} (\n");
        result.push_str(&split_inner);
        result.push_str(&format!("{base_pad}){suffix}\n"));
        Some(result)
    }

    fn split_condition_logical(&self, formatted: &str, line_pad: &str) -> Option<String> {
        for op in [BinaryOp::Or, BinaryOp::And, BinaryOp::Concat] {
            let positions = find_top_level_binary_op(formatted, op);
            if positions.is_empty() {
                continue;
            }
            let operands = logical_operands(formatted, op, &positions)?;
            let token = op.token();
            let mut result = format!("{line_pad}{}\n", operands[0]);
            for operand in &operands[1..] {
                self.append_logical_operand(&mut result, token, operand, line_pad);
            }
            return Some(result);
        }
        None
    }

    fn split_parenthesized_expression(&self, formatted: &str, base_pad: &str) -> Option<String> {
        let trimmed = formatted.trim();
        if !trimmed.starts_with('(') {
            return None;
        }
        let chars: Vec<char> = trimmed.chars().collect();
        let close_pos = find_matching_close(&chars, 0)?;
        if close_pos + 1 != chars.len() {
            return None;
        }
        let inner: String = chars[1..close_pos].iter().collect();
        let inner_pad = format!("{base_pad}{}", self.indent);
        let split_inner = self
            .split_condition_logical(inner.trim(), &inner_pad)
            .or_else(|| self.split_long_line(inner.trim(), &inner_pad))
            .unwrap_or_else(|| format!("{inner_pad}{inner}\n"));
        let mut result = format!("{base_pad}(\n");
        result.push_str(&split_inner);
        result.push_str(&format!("{base_pad})\n"));
        Some(result)
    }

    fn split_long_by_logical(&self, formatted: &str, base_pad: &str) -> Option<String> {
        for op in [BinaryOp::Or, BinaryOp::And, BinaryOp::Concat] {
            let positions = find_top_level_binary_op(formatted, op);
            if positions.is_empty() {
                continue;
            }
            if let Some(split) = self.build_logical_split(formatted, base_pad, op, &positions) {
                return Some(split);
            }
        }
        None
    }

    fn build_logical_split(
        &self,
        formatted: &str,
        base_pad: &str,
        op: BinaryOp,
        positions: &[usize],
    ) -> Option<String> {
        let token = op.token();
        let operands = logical_operands(formatted, op, positions)?;
        let inner_pad = format!("{base_pad}{}", self.indent);
        let mut result = format!("{base_pad}{}\n", operands[0]);
        for operand in &operands[1..] {
            self.append_logical_operand(&mut result, token, operand, &inner_pad);
        }
        Some(result)
    }

    fn append_logical_operand(&self, result: &mut String, token: &str, operand: &str, inner_pad: &str) {
        let single = format!("{inner_pad}{token} {operand}");
        if visual_len(&single) <= self.max_line_length {
            result.push_str(&format!("{single}\n"));
            return;
        }
        if let Some(split) = self.split_parenthesized_expression(operand, inner_pad) {
            let mut lines = split.lines();
            if let Some(first) = lines.next() {
                let first = first.strip_prefix(inner_pad).unwrap_or(first).trim_start();
                result.push_str(&format!("{inner_pad}{token} {first}\n"));
                for line in lines {
                    result.push_str(line);
                    result.push('\n');
                }
                return;
            }
        }
        result.push_str(&format!("{single}\n"));
    }

    fn format_assignment_array_item(&self, item: &str, pad: &str) -> String {
        let indent = &self.indent;
        let item = item.trim();
        if item.is_empty() {
            return String::new();
        }

        if item.starts_with('[') && item.ends_with(']') {
            if let Some(expanded) = self.expand_assignment_array_literal(item, pad) {
                return expanded;
            }
            return format!("{pad}{item},\n");
        }

        if let Some(expanded) = self.expand_inline_closure(item, pad) {
            return expanded;
        }

        if let Some(arrow_pos) = find_top_level_fat_arrow(item) {
            let key = item[..arrow_pos + 2].trim_end();
            let value = item[arrow_pos + 2..].trim_start();
            if value.starts_with('[') && value.ends_with(']') {
                let inner = &value[1..value.len() - 1];
                let sub_items = split_by_commas(inner);
                let single_nested_array = sub_items.len() == 1 && sub_items[0].trim_start().starts_with('[');
                if sub_items.len() > 1 || single_nested_array || sub_items.iter().any(|s| has_expandable_closure(s)) {
                    let nested_pad = format!("{pad}{indent}");
                    let mut result = format!("{pad}{key} [\n");
                    for sub in &sub_items {
                        result.push_str(&self.format_assignment_array_item(sub, &nested_pad));
                    }
                    result.push_str(&format!("{pad}],\n"));
                    return result;
                }
            }
        }

        if visual_len(pad) + visual_len(item) + 1 > self.max_line_length {
            if let Some(split) = self.try_split_long_line(item, pad) {
                let trimmed = split.trim_end_matches('\n');
                return format!("{trimmed},\n");
            }
        }

        format!("{pad}{item},\n")
    }

    fn expand_assignment_array_literal(&self, array: &str, pad: &str) -> Option<String> {
        let trimmed = array.trim();
        if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
            return None;
        }

        let inner = &trimmed[1..trimmed.len() - 1];
        let items = split_by_commas(inner);
        if items.is_empty() {
            return None;
        }

        let first = items[0].trim();
        let should_expand = items.len() > 1
            || first.starts_with('[')
            || items.iter().any(|it| has_expandable_closure(it))
            || visual_len(pad) + visual_len(&self.indent) + visual_len(first) + 1 > self.max_line_length;
        if !should_expand {
            return None;
        }

        let nested_pad = format!("{pad}{}", self.indent);
        let mut result = format!("{pad}[\n");
        for item in &items {
            result.push_str(&self.format_assignment_array_item(item, &nested_pad));
        }
        result.push_str(&format!("{pad}],\n"));
        Some(result)
    }

    fn expand_assignment_array(&self, formatted: &str, base_pad: &str) -> Option<String> {
        let eq_pos = find_top_level_assignment_equal(formatted)?;
        let lhs = formatted[..=eq_pos].trim_end();
        let rhs = formatted[eq_pos + 1..].trim_start();

        let rhs_trimmed = rhs.trim();
        let (array_part, suffix) = if let Some(no_semicolon) = rhs_trimmed.strip_suffix(';') {
            (no_semicolon.trim_end(), ";")
        } else {
            (rhs_trimmed, "")
        };

        if !array_part.starts_with('[') || !array_part.ends_with(']') {
            return None;
        }

        let inner = &array_part[1..array_part.len() - 1];
        let items = split_by_commas(inner);
        if items.is_empty() {
            return None;
        }

        let first = items[0].trim();
        let should_expand = items.len() > 1
            || first.starts_with('[')
            || items.iter().any(|it| has_expandable_closure(it))
            || visual_len(base_pad) + visual_len(lhs) + 2 + visual_len(array_part) > self.max_line_length;
        if !should_expand {
            return None;
        }

        let nested_pad = format!("{base_pad}{}", self.indent);
        let mut result = format!("{base_pad}{lhs} [\n");
        for item in &items {
            result.push_str(&self.format_assignment_array_item(item, &nested_pad));
        }
        result.push_str(&format!("{base_pad}]{suffix}\n"));
        Some(result)
    }

    pub(crate) fn build_split(&self, prefix: &str, args: &[String], suffix: &str, pad: &str) -> String {
        let inner_pad = format!("{pad}{}", self.indent);
        let mut result = String::new();
        let prefix_trimmed = prefix.trim();

        self.emit_split_prefix(prefix_trimmed, pad, &mut result);
        for arg in args {
            result.push_str(&self.format_split_arg(arg, &inner_pad));
        }
        self.emit_split_suffix(prefix_trimmed, suffix, pad, &mut result);
        result
    }

    fn emit_split_prefix(&self, prefix_trimmed: &str, pad: &str, result: &mut String) {
        if visual_len(pad) + visual_len(prefix_trimmed) > self.max_line_length {
            let mut prefix_parts = split_by_commas(prefix_trimmed);
            if prefix_parts.len() > 1 {
                let last = prefix_parts.pop().unwrap_or_default();
                for part in prefix_parts {
                    result.push_str(&format!("{pad}{},\n", part.trim()));
                }
                result.push_str(&format!("{pad}{}\n", last.trim()));
                return;
            }
        }
        result.push_str(&format!("{pad}{prefix_trimmed}\n"));
    }

    fn format_split_arg(&self, arg: &str, inner_pad: &str) -> String {
        let line_len = visual_len(inner_pad) + visual_len(arg) + 1;
        if line_len > self.max_line_length {
            if let Some(expanded) = self.expand_nested_array(arg, inner_pad) {
                return expanded;
            }
            if let Some(expanded) = self.expand_bare_array(arg, inner_pad) {
                return expanded;
            }
            if let Some(expanded) = self.expand_inline_closure(arg, inner_pad) {
                return expanded;
            }
            if let Some(split) = self.try_split_long_line(arg, inner_pad) {
                return format!("{},\n", split.trim_end_matches('\n'));
            }
        }
        if let Some(expanded) = self.expand_bare_array(arg, inner_pad) {
            return expanded;
        }
        if let Some(expanded) = self.expand_closure_element(arg, inner_pad) {
            return expanded;
        }
        if let Some(expanded) = self.expand_list_value(arg, inner_pad) {
            return expanded;
        }
        format!("{inner_pad}{arg},\n")
    }

    fn emit_split_suffix(&self, prefix_trimmed: &str, suffix: &str, pad: &str, result: &mut String) {
        let suffix_trimmed = suffix.trim();
        let initial_depth = bracket_balance(prefix_trimmed);
        let split_depth = initial_depth - count_leading_closers(suffix_trimmed) as i32;
        if let Some(split) = self.split_long_by_commas_from_depth(suffix_trimmed, pad, initial_depth, split_depth) {
            result.push_str(&split);
            return;
        }
        if visual_len(pad) + visual_len(suffix_trimmed) > self.max_line_length {
            if let Some(split) = self.try_split_long_line(suffix_trimmed, pad) {
                result.push_str(&split);
                return;
            }
        }
        result.push_str(&format!("{pad}{suffix_trimmed}\n"));
    }

    fn split_long_by_commas(&self, formatted: &str, pad: &str) -> Option<String> {
        self.split_long_by_commas_from_depth(formatted, pad, 0, 0)
    }

    fn split_long_by_commas_from_depth(
        &self,
        formatted: &str,
        pad: &str,
        start_depth: i32,
        split_depth: i32,
    ) -> Option<String> {
        let parts = split_by_commas_with_depth(formatted, start_depth, split_depth);
        if parts.len() <= 1 {
            return None;
        }

        let mut result = String::new();
        for (idx, part) in parts.iter().enumerate() {
            let mut line = part.trim().to_string();
            if idx < parts.len() - 1 {
                line.push(',');
            }

            if visual_len(pad) + visual_len(&line) > self.max_line_length {
                if let Some(expanded) = self.expand_bare_array_with_suffix(&line, pad) {
                    result.push_str(&expanded);
                    continue;
                }
                if let Some(expanded) = self.expand_nested_array(&line, pad) {
                    result.push_str(&expanded);
                    continue;
                }
                if let Some(expanded) = self.expand_bare_array(&line, pad) {
                    result.push_str(&expanded);
                    continue;
                }
                if let Some(expanded) = self.expand_inline_closure(&line, pad) {
                    result.push_str(&expanded);
                    continue;
                }
                if let Some(split) = self.try_split_long_line(&line, pad) {
                    result.push_str(&split);
                    continue;
                }
            }

            result.push_str(&format!("{pad}{line}\n"));
        }

        Some(result)
    }

    fn expand_bare_array_with_suffix(&self, line: &str, pad: &str) -> Option<String> {
        let trimmed = line.trim();
        let (array_part, suffix) = if let Some(s) = trimmed.strip_suffix(',') {
            (s.trim_end(), ",")
        } else if let Some(s) = trimmed.strip_suffix(';') {
            (s.trim_end(), ";")
        } else {
            (trimmed, "")
        };

        let mut expanded = self.expand_bare_array(array_part, pad)?;

        match suffix {
            "," => Some(expanded),
            ";" => {
                if expanded.ends_with(",\n") {
                    expanded.truncate(expanded.len() - 2);
                    expanded.push_str(";\n");
                }
                Some(expanded)
            }
            "" => {
                if expanded.ends_with(",\n") {
                    expanded.truncate(expanded.len() - 2);
                    expanded.push('\n');
                }
                Some(expanded)
            }
            _ => None,
        }
    }

    pub(crate) fn expand_bare_array(&self, arg: &str, pad: &str) -> Option<String> {
        let trimmed = arg.trim();
        if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
            return None;
        }
        let inner = &trimmed[1..trimmed.len() - 1];
        let items = split_by_commas(inner);
        if items.len() <= 1 {
            return self.expand_singleton_array(&items, pad);
        }
        let nested_pad = format!("{pad}{}", self.indent);
        let mut result = format!("{pad}[\n");
        for item in &items {
            result.push_str(&self.format_bare_array_item(item, &nested_pad));
        }
        result.push_str(&format!("{pad}],\n"));
        Some(result)
    }

    fn expand_singleton_array(&self, items: &[String], pad: &str) -> Option<String> {
        if items.len() != 1 {
            return None;
        }
        let item = &items[0];
        let nested_pad = format!("{pad}{}", self.indent);
        let item_line_len = visual_len(&nested_pad) + visual_len(item) + 1;
        if item_line_len <= self.max_line_length && !has_expandable_closure(item) {
            return None;
        }
        let mut result = format!("{pad}[\n");
        if let Some(expanded) = self.expand_nested_array(item, &nested_pad) {
            result.push_str(&expanded);
        } else if let Some(expanded) = self.expand_bare_sub_array(item, &nested_pad) {
            result.push_str(&expanded);
        } else if let Some(expanded) = self.expand_inline_closure(item, &nested_pad) {
            result.push_str(&expanded);
        } else if let Some(split) = self.try_split_long_line(item, &nested_pad) {
            result.push_str(split.trim_end_matches('\n'));
            result.push_str(",\n");
        } else {
            result.push_str(&format!("{nested_pad}{item},\n"));
        }
        result.push_str(&format!("{pad}],\n"));
        Some(result)
    }

    fn format_bare_array_item(&self, item: &str, nested_pad: &str) -> String {
        let item_line_len = visual_len(nested_pad) + visual_len(item) + 1;
        if item_line_len > self.max_line_length
            && let Some(expanded) = self.expand_nested_array(item, nested_pad)
        {
            return expanded;
        }
        if item.starts_with('[') && item.ends_with(']') {
            if let Some(expanded) = self.expand_bare_sub_array(item, nested_pad) {
                return expanded;
            }
        }
        if let Some(expanded) = self.expand_closure_element(item, nested_pad) {
            return expanded;
        }
        format!("{nested_pad}{item},\n")
    }

    pub(crate) fn expand_bare_sub_array(&self, item: &str, pad: &str) -> Option<String> {
        if !item.starts_with('[') || !item.ends_with(']') {
            return None;
        }
        let sub_inner = &item[1..item.len() - 1];
        let sub_items = split_by_commas(sub_inner);
        if sub_items.len() <= 1 && !sub_items.iter().any(|s| has_expandable_closure(s)) {
            return None;
        }
        let deeper_pad = format!("{pad}{}", self.indent);
        let mut result = format!("{pad}[\n");
        for sub in &sub_items {
            let sub_line_len = visual_len(&deeper_pad) + visual_len(sub) + 1;
            if sub_line_len > self.max_line_length {
                if let Some(expanded) = self.expand_nested_array(sub, &deeper_pad) {
                    result.push_str(&expanded);
                    continue;
                }
                if let Some(expanded) = self.expand_inline_closure(sub, &deeper_pad) {
                    result.push_str(&expanded);
                    continue;
                }
                if let Some(split) = self.try_split_long_line(sub, &deeper_pad) {
                    let trimmed = split.trim_end_matches('\n');
                    result.push_str(trimmed);
                    result.push_str(",\n");
                    continue;
                }
            }
            if let Some(expanded) = self.expand_inline_closure(sub, &deeper_pad) {
                result.push_str(&expanded);
                continue;
            }
            result.push_str(&format!("{deeper_pad}{sub},\n"));
        }
        result.push_str(&format!("{pad}],\n"));
        Some(result)
    }

    pub(crate) fn expand_brace_block(&self, stmt: &str, pad: &str) -> Option<String> {
        let (open, close) = find_brace_block(stmt)?;
        let chars: Vec<char> = stmt.chars().collect();
        let body: String = chars[open + 1..close].iter().collect();
        let body = body.trim();
        if body.is_empty() {
            return None;
        }
        let header: String = chars[..open].iter().collect();
        let header = header.trim_end();
        let after: String = chars[close + 1..].iter().collect();
        let after = after.trim();
        let inner_pad = format!("{pad}{}", self.indent);
        let body_stmts = normalize_closure_body(body);
        if body_stmts.is_empty() {
            return None;
        }
        let mut result = format!("{pad}{header} {{\n");
        for s in &body_stmts {
            let line_len = visual_len(&inner_pad) + visual_len(s);
            if line_len > self.max_line_length {
                if let Some(split) = self.try_split_long_line(s, &inner_pad) {
                    result.push_str(&split);
                    continue;
                }
            }
            result.push_str(&format!("{inner_pad}{s}\n"));
        }
        if after.is_empty() {
            result.push_str(&format!("{pad}}}\n"));
        } else {
            result.push_str(&format!("{pad}}} {after}\n"));
        }
        Some(result)
    }

    pub(crate) fn expand_braced_value(&self, expr: &str, pad: &str) -> Option<String> {
        let (open, close) = find_brace_block(expr)?;
        let chars: Vec<char> = expr.chars().collect();
        let body: String = chars[open + 1..close].iter().collect();
        let body = body.trim();
        if body.is_empty() || count_top_level_semicolons(body) > 0 {
            return None;
        }
        let arms = split_by_commas(body);
        if arms.len() <= 1 {
            return None;
        }
        let header: String = chars[..open].iter().collect();
        let header = header.trim_end();
        let suffix: String = chars[close + 1..].iter().collect();
        let suffix = suffix.trim();
        let inner_pad = format!("{pad}{}", self.indent);
        let mut result = format!("{pad}{header} {{\n");
        for arm in &arms {
            if visual_len(&inner_pad) + visual_len(arm) + 1 > self.max_line_length
                && let Some(split) = self.try_split_long_line(arm, &inner_pad)
            {
                result.push_str(split.trim_end_matches('\n'));
                result.push_str(",\n");
                continue;
            }
            result.push_str(&format!("{inner_pad}{arm},\n"));
        }
        result.push_str(&format!("{pad}}}{suffix}\n"));
        Some(result)
    }

    pub(crate) fn expand_closure_element(&self, item: &str, pad: &str) -> Option<String> {
        if !has_expandable_closure(item) {
            return None;
        }
        self.expand_nested_array(item, pad)
            .or_else(|| self.expand_bare_sub_array(item, pad))
            .or_else(|| self.expand_inline_closure(item, pad))
    }

    pub(crate) fn expand_inline_closure(&self, arg: &str, pad: &str) -> Option<String> {
        let (open_brace, close_brace) = find_closure_body(arg)?;
        let chars: Vec<char> = arg.chars().collect();
        let body: String = chars[open_brace + 1..close_brace].iter().collect();
        let stmts = normalize_closure_body(&body);
        if stmts.is_empty() {
            return None;
        }
        let header: String = chars[..open_brace].iter().collect();
        let header = header.trim_end();
        if bracket_balance(header) != 0 {
            return None;
        }
        let after_close: String = chars[close_brace + 1..].iter().collect();
        let after_close = after_close.trim_start();
        let body_pad = format!("{pad}{}", self.indent);
        let mut result = format!("{pad}{header} {{\n");
        for stmt in &stmts {
            if let Some(expanded) = self.expand_brace_block(stmt, &body_pad) {
                result.push_str(&expanded);
                continue;
            }
            let line_len = visual_len(&body_pad) + visual_len(stmt);
            if line_len > self.max_line_length {
                if let Some(split) = self.try_split_long_line(stmt, &body_pad) {
                    result.push_str(&split);
                    continue;
                }
            }
            result.push_str(&format!("{body_pad}{stmt}\n"));
        }
        if after_close.is_empty() {
            result.push_str(&format!("{pad}}},\n"));
        } else {
            result.push_str(&format!("{pad}}}{after_close}\n"));
        }
        Some(result)
    }

    pub(crate) fn expand_list_value(&self, arg: &str, pad: &str) -> Option<String> {
        let (skip, arrow_pos) = find_array_arrow(arg)?;
        let value = arg[skip + arrow_pos + 2..].trim();
        let inner = value.strip_prefix('[')?.strip_suffix(']')?;
        if !array_is_list(inner) {
            return None;
        }
        self.expand_nested_array(arg, pad)
    }

    pub(crate) fn expand_nested_array(&self, arg: &str, pad: &str) -> Option<String> {
        let (skip, arrow_pos) = find_array_arrow(arg)?;
        let key = &arg[..skip + arrow_pos + 2];
        if key.trim_start().starts_with('[') {
            return None;
        }
        let value = arg[skip + arrow_pos + 2..].trim();
        if !value.starts_with('[') || !value.ends_with(']') {
            return None;
        }
        let inner = &value[1..value.len() - 1];
        let items = split_by_commas(inner);
        let single_nested_array = items.len() == 1 && items[0].trim_start().starts_with('[');
        let has_closure = items.iter().any(|it| has_expandable_closure(it));
        if items.len() <= 1 && !single_nested_array && !has_closure {
            return None;
        }
        let nested_pad = format!("{pad}{}", self.indent);
        let mut result = format!("{pad}{key} [\n");
        for item in &items {
            if visual_len(&nested_pad) + visual_len(item) + 1 > self.max_line_length {
                if let Some(expanded) = self.expand_nested_array(item, &nested_pad) {
                    result.push_str(&expanded);
                    continue;
                }
                if let Some(bare) = self.expand_bare_sub_array(item, &nested_pad) {
                    result.push_str(&bare);
                    continue;
                }
                if let Some(expanded) = self.expand_inline_closure(item, &nested_pad) {
                    result.push_str(&expanded);
                    continue;
                }
                if let Some(split) = self.try_split_long_line(item, &nested_pad) {
                    let trimmed = split.trim_end_matches('\n');
                    result.push_str(trimmed);
                    result.push_str(",\n");
                    continue;
                }
            }
            if let Some(bare) = self.expand_bare_sub_array(item, &nested_pad) {
                result.push_str(&bare);
                continue;
            }
            if let Some(expanded) = self.expand_inline_closure(item, &nested_pad) {
                result.push_str(&expanded);
                continue;
            }
            result.push_str(&format!("{nested_pad}{item},\n"));
        }
        result.push_str(&format!("{pad}],\n"));
        Some(result)
    }
}

fn logical_roundtrip_ok(original: &str, operands: &[String], token: &str) -> bool {
    let rebuilt = operands.join(&format!(" {token} "));
    strip_whitespace(&rebuilt) == strip_whitespace(original)
}

fn strip_whitespace(code: &str) -> String {
    code.chars().filter(|c| !c.is_whitespace()).collect()
}
