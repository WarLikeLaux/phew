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
        let tokens = lexer::tokenize(source);
        let nodes = ast::parse(tokens);
        self.format(&nodes)
    }
}

impl Default for Formatter {
    fn default() -> Self {
        Self::new(&Config::default())
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
