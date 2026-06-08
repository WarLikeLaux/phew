use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum IndentStyle {
    #[default]
    Spaces,
    Tabs,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub indent_style: IndentStyle,
    pub indent_size: usize,
    pub max_line_length: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            indent_style: IndentStyle::Spaces,
            indent_size: 4,
            max_line_length: 120,
        }
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("не удалось прочитать конфиг {path}: {source}")]
    Read { path: PathBuf, source: std::io::Error },
    #[error("ошибка разбора конфига {path}: {source}")]
    Parse { path: PathBuf, source: toml::de::Error },
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let text = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        toml::from_str(&text).map_err(|source| ConfigError::Parse {
            path: path.to_path_buf(),
            source,
        })
    }

    pub fn discover(start: &Path) -> Option<PathBuf> {
        let mut dir = Some(start);
        while let Some(current) = dir {
            let candidate = current.join(".phew.toml");
            if candidate.is_file() {
                return Some(candidate);
            }
            if current.join(".git").exists() {
                return None;
            }
            dir = current.parent();
        }
        None
    }

    pub fn indent_string(&self) -> String {
        match self.indent_style {
            IndentStyle::Spaces => " ".repeat(self.indent_size),
            IndentStyle::Tabs => "\t".to_string(),
        }
    }

    pub fn default_toml() -> &'static str {
        concat!(
            "# Конфигурация phew (https://github.com/WarLikeLaux/phew)\n",
            "\n",
            "# Стиль отступа: \"spaces\" или \"tabs\"\n",
            "indent_style = \"spaces\"\n",
            "\n",
            "# Размер отступа в пробелах (игнорируется при indent_style = \"tabs\")\n",
            "indent_size = 4\n",
            "\n",
            "# Целевая максимальная длина строки\n",
            "max_line_length = 120\n",
        )
    }
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
