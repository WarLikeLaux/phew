pub mod docblock;
pub mod echo;
pub mod engine;
pub mod html;
pub mod indent;
pub mod php;
pub mod split;
pub mod yii;

use crate::config::Config;

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
}

impl Default for Formatter {
    fn default() -> Self {
        Self::new(&Config::default())
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
