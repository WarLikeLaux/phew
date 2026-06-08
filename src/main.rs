use std::path::{Path, PathBuf};

use clap::Parser;
use rayon::prelude::*;

use phew::config::{Config, IndentStyle};
use phew::formatter::Formatter;
use phew::io::walker;
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

fn render_tree(nodes: &[ast::Node], indent: usize, out: &mut String) {
    let pad = "  ".repeat(indent);
    for node in nodes {
        match node {
            ast::Node::Element {
                name,
                attributes,
                children,
            } => {
                if attributes.is_empty() {
                    out.push_str(&format!("{pad}<{name}>\n"));
                } else {
                    let attrs: Vec<String> = attributes
                        .iter()
                        .map(|a| match &a.value {
                            Some(v) => format!("{}=\"{}\"", a.name, v),
                            None => a.name.clone(),
                        })
                        .collect();
                    out.push_str(&format!("{pad}<{name} {}>\n", attrs.join(" ")));
                }
                render_tree(children, indent + 1, out);
            }
            ast::Node::Text(s) => {
                let trimmed = s.trim();
                if !trimmed.is_empty() {
                    out.push_str(&format!("{pad}TEXT: {trimmed:?}\n"));
                }
            }
            ast::Node::PhpBlock(s) => out.push_str(&format!("{pad}PHP: <?php {s} ?>\n")),
            ast::Node::PhpEcho(s) => out.push_str(&format!("{pad}PHP: <?= {s} ?>\n")),
            ast::Node::Doctype(s) => out.push_str(&format!("{pad}DOCTYPE: {s}\n")),
            ast::Node::Comment(s) => out.push_str(&format!("{pad}COMMENT: {s}\n")),
        }
    }
}

fn process_file(path: &Path, cli: &Cli, formatter: &Formatter) -> String {
    let display = path.display();
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading {display}: {e}");
            return String::new();
        }
    };

    let tokens = lexer::tokenize(&content);

    if cli.tokens {
        let mut out = format!("=== {display} ===\n");
        for token in &tokens {
            out.push_str(&format!("{token:?}\n"));
        }
        return out;
    }

    if cli.tree {
        let nodes = ast::parse(tokens);
        let mut out = format!("=== {display} ===\n");
        render_tree(&nodes, 0, &mut out);
        return out;
    }

    let nodes = ast::parse(tokens);
    let formatted = formatter.format(&nodes);
    if cli.write {
        if let Err(e) = std::fs::write(path, &formatted) {
            eprintln!("Error writing {display}: {e}");
        }
        return String::new();
    }
    formatted
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

    let files = walker::collect_files(&cli.paths);

    let outputs: Vec<String> = files
        .par_iter()
        .map(|path| process_file(path, &cli, &formatter))
        .collect();

    for output in &outputs {
        print!("{output}");
    }

    Ok(())
}
