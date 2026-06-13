pub mod attrs;
pub mod declaration;
pub mod docblock;
pub mod echo;
pub mod engine;
pub mod indent;
pub mod normalize;
pub mod php;
pub mod php_emit;
pub mod scan;
pub mod split;

use crate::config::Config;
use crate::parser::{ast, lexer};

const MAX_FORMAT_PASSES: usize = 4;

#[derive(Debug)]
pub struct Formatter {
    indent: String,
    max_line_length: usize,
}

impl Formatter {
    pub fn new(cfg: &Config) -> Self {
        Self {
            indent: cfg.indent_string(),
            max_line_length: cfg.max_line_length,
        }
    }

    pub fn format_source(&self, source: &str) -> String {
        let source = source.strip_prefix('\u{FEFF}').unwrap_or(source);
        if is_php_codegen_template(source) {
            return source.to_string();
        }
        let mut current = self.format_once(source);
        for _ in 1..MAX_FORMAT_PASSES {
            let next = self.format_once(&current);
            if next == current {
                return current;
            }
            current = next;
        }
        current
    }

    fn format_once(&self, source: &str) -> String {
        let tokens = lexer::tokenize(source);
        let nodes = ast::parse(tokens);
        self.format(&nodes)
    }
}

fn is_php_codegen_template(source: &str) -> bool {
    source.contains("echo \"<?php\\n\";")
}

impl Default for Formatter {
    fn default() -> Self {
        Self::new(&Config::default())
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
