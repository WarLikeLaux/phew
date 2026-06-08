use std::path::PathBuf;

use clap::Parser;
use phew::config::{Config, IndentStyle};
use phew::formatter::Formatter;
use phew::parser::{ast, lexer};

#[derive(Parser)]
#[command(name = "phew")]
#[command(about = "Fast HTML + PHP formatter for Yii 2 view files")]
struct Cli {
    #[arg(help = "Files or directories to format")]
    paths: Vec<String>,

    #[arg(short, long, help = "Write result back to file")]
    write: bool,

    #[arg(long, help = "Show tokens instead of formatting")]
    tokens: bool,

    #[arg(long, help = "Show AST tree")]
    tree: bool,

    #[arg(long, value_name = "PATH", help = "Path to a .phew.toml config file")]
    config: Option<PathBuf>,

    #[arg(long, value_name = "N", help = "Override target line length")]
    line_length: Option<usize>,

    #[arg(long, value_enum, help = "Override indent style")]
    indent_style: Option<IndentStyle>,

    #[arg(long, value_name = "N", help = "Override indent size (spaces)")]
    indent_size: Option<usize>,

    #[arg(long, help = "Write a default .phew.toml to the current directory")]
    init: bool,
}

/// Собирает итоговую конфигурацию: файл (`--config` или авто-поиск) поверх
/// дефолтов, затем точечные CLI-оверрайды.
fn resolve_config(cli: &Cli) -> anyhow::Result<Config> {
    let mut config = match &cli.config {
        Some(path) => Config::load(path)?,
        None => {
            let cwd = std::env::current_dir()?;
            match Config::discover(&cwd) {
                Some(path) => Config::load(&path)?,
                None => Config::default(),
            }
        }
    };

    if let Some(style) = cli.indent_style {
        config.indent_style = style;
    }
    if let Some(size) = cli.indent_size {
        config.indent_size = size;
    }
    if let Some(len) = cli.line_length {
        config.max_line_length = len;
    }

    Ok(config)
}

/// Записывает `.phew.toml` со значениями по умолчанию, не перезаписывая
/// существующий файл.
fn run_init() -> anyhow::Result<()> {
    let path = std::path::Path::new(".phew.toml");
    if path.exists() {
        println!(".phew.toml уже существует — оставлен без изменений");
        return Ok(());
    }
    std::fs::write(path, Config::default_toml())?;
    println!("Создан .phew.toml со значениями по умолчанию");
    Ok(())
}

fn print_tree(nodes: &[ast::Node], indent: usize) {
    let pad = "  ".repeat(indent);
    for node in nodes {
        match node {
            ast::Node::Element {
                name,
                attributes,
                children,
            } => {
                if attributes.is_empty() {
                    println!("{pad}<{name}>");
                } else {
                    let attrs: Vec<String> = attributes
                        .iter()
                        .map(|a| match &a.value {
                            Some(v) => format!("{}=\"{}\"", a.name, v),
                            None => a.name.clone(),
                        })
                        .collect();
                    println!("{pad}<{name} {}>", attrs.join(" "));
                }
                print_tree(children, indent + 1);
            }
            ast::Node::Text(s) => {
                let trimmed = s.trim();
                if !trimmed.is_empty() {
                    println!("{pad}TEXT: {trimmed:?}");
                }
            }
            ast::Node::PhpBlock(s) => println!("{pad}PHP: <?php {s} ?>"),
            ast::Node::PhpEcho(s) => println!("{pad}PHP: <?= {s} ?>"),
            ast::Node::Doctype(s) => println!("{pad}DOCTYPE: {s}"),
            ast::Node::Comment(s) => println!("{pad}COMMENT: {s}"),
        }
    }
}

fn process_file(path: &str, cli: &Cli, formatter: &Formatter) {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading {path}: {e}");
            return;
        }
    };

    let tokens = lexer::tokenize(&content);

    if cli.tokens {
        println!("=== {path} ===");
        for token in &tokens {
            println!("{token:?}");
        }
    } else if cli.tree {
        let nodes = ast::parse(tokens);
        println!("=== {path} ===");
        print_tree(&nodes, 0);
    } else {
        let nodes = ast::parse(tokens);
        let formatted = formatter.format(&nodes);
        if cli.write {
            if let Err(e) = std::fs::write(path, &formatted) {
                eprintln!("Error writing {path}: {e}");
            }
        } else {
            print!("{formatted}");
        }
    }
}

fn main() -> anyhow::Result<()> {
    if std::env::var_os("RUST_BACKTRACE").is_none() {
        std::panic::set_hook(Box::new(|_| {}));
    }

    let cli = Cli::parse();

    if cli.init {
        return run_init();
    }

    if cli.paths.is_empty() {
        println!("phew v{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let config = resolve_config(&cli)?;
    let formatter = Formatter::new(&config);

    let mut files: Vec<String> = Vec::new();
    for path in &cli.paths {
        let meta = std::fs::metadata(path);
        if let Ok(m) = &meta
            && m.is_dir()
        {
            collect_files(path, &mut files);
            continue;
        }
        files.push(path.clone());
    }

    for path in &files {
        process_file(path, &cli, &formatter);
    }

    Ok(())
}

fn collect_files(dir: &str, out: &mut Vec<String>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Error reading {dir}: {e}");
            return;
        }
    };
    let mut paths: Vec<_> = entries.filter_map(|e| e.ok()).collect();
    paths.sort_by_key(|e| e.path());
    for entry in paths {
        let path = entry.path();
        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        if metadata.is_dir() {
            if !metadata.file_type().is_symlink() {
                collect_files(&path.to_string_lossy(), out);
            }
        } else if let Some(ext) = path.extension() {
            let ext = ext.to_string_lossy();
            if ext == "php" || ext == "html" {
                out.push(path.to_string_lossy().to_string());
            }
        }
    }
}
