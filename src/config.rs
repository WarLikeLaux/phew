use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error;

/// Стиль отступа: пробелы или табуляция.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum IndentStyle {
    #[default]
    Spaces,
    Tabs,
}

/// Параметры форматирования, читаемые из `.phew.toml`.
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

/// Ошибки чтения и разбора файла конфигурации.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("не удалось прочитать конфиг {path}: {source}")]
    Read { path: PathBuf, source: std::io::Error },
    #[error("ошибка разбора конфига {path}: {source}")]
    Parse { path: PathBuf, source: toml::de::Error },
}

impl Config {
    /// Загружает конфигурацию из файла, расположенного по пути `path`.
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

    /// Ищет `.phew.toml`, поднимаясь от `start` вверх по дереву каталогов.
    ///
    /// Подъём останавливается на каталоге с `.git` (граница репозитория) либо на
    /// корне файловой системы, чтобы не подхватить чужой конфиг извне проекта.
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

    /// Возвращает строку одного уровня отступа согласно стилю и размеру.
    pub fn indent_string(&self) -> String {
        match self.indent_style {
            IndentStyle::Spaces => " ".repeat(self.indent_size),
            IndentStyle::Tabs => "\t".to_string(),
        }
    }

    /// Текст файла `.phew.toml` со значениями по умолчанию для `--init`.
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
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn unique_dir(tag: &str) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("phew_cfg_{tag}_{}_{n}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn default_matches_legacy_constants() {
        let cfg = Config::default();
        assert_eq!(cfg.indent_style, IndentStyle::Spaces);
        assert_eq!(cfg.indent_size, 4);
        assert_eq!(cfg.max_line_length, 120);
    }

    #[test]
    fn indent_string_spaces() {
        let cfg = Config {
            indent_size: 2,
            ..Config::default()
        };
        assert_eq!(cfg.indent_string(), "  ");
    }

    #[test]
    fn indent_string_tabs() {
        let cfg = Config {
            indent_style: IndentStyle::Tabs,
            ..Config::default()
        };
        assert_eq!(cfg.indent_string(), "\t");
    }

    #[test]
    fn parse_full_config() {
        let cfg: Config = toml::from_str("indent_style = \"tabs\"\nindent_size = 8\nmax_line_length = 100\n").unwrap();
        assert_eq!(cfg.indent_style, IndentStyle::Tabs);
        assert_eq!(cfg.indent_size, 8);
        assert_eq!(cfg.max_line_length, 100);
    }

    #[test]
    fn parse_partial_fills_defaults() {
        let cfg: Config = toml::from_str("max_line_length = 80\n").unwrap();
        assert_eq!(cfg.indent_style, IndentStyle::Spaces);
        assert_eq!(cfg.indent_size, 4);
        assert_eq!(cfg.max_line_length, 80);
    }

    #[test]
    fn parse_rejects_unknown_field() {
        let result: Result<Config, _> = toml::from_str("tab_width = 4\n");
        assert!(result.is_err());
    }

    #[test]
    fn load_reads_file() {
        let dir = unique_dir("load");
        let path = dir.join(".phew.toml");
        std::fs::write(&path, "indent_size = 3\n").unwrap();

        let cfg = Config::load(&path).unwrap();
        assert_eq!(cfg.indent_size, 3);

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn load_missing_file_is_read_error() {
        let path = unique_dir("missing").join("absent.toml");
        let err = Config::load(&path).unwrap_err();
        assert!(matches!(err, ConfigError::Read { .. }));
    }

    #[test]
    fn discover_finds_config_walking_up() {
        let root = unique_dir("discover_up");
        let nested = root.join("a").join("b");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(root.join(".phew.toml"), "indent_size = 2\n").unwrap();

        let found = Config::discover(&nested).unwrap();
        assert_eq!(found, root.join(".phew.toml"));

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn discover_stops_at_git_boundary() {
        let outer = unique_dir("discover_git");
        std::fs::write(outer.join(".phew.toml"), "indent_size = 2\n").unwrap();
        let repo = outer.join("repo");
        std::fs::create_dir_all(repo.join(".git")).unwrap();

        assert!(Config::discover(&repo).is_none());

        std::fs::remove_dir_all(&outer).unwrap();
    }
}
