pub mod attrs;
pub mod docblock;
pub mod echo;
pub mod engine;
pub mod html;
pub mod indent;
pub mod php;
pub mod php_emit;
pub mod scan;
pub mod split;
pub mod yii;

use crate::config::Config;
use crate::parser::{ast, lexer};

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
