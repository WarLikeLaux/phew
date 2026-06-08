use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{ArgGroup, Parser};
use rayon::prelude::*;
use similar::TextDiff;

use phew::config::{Config, IndentStyle};
use phew::formatter::Formatter;
use phew::io::walker;
use phew::parser::{ast, lexer};

const STDIN_MARKER: &str = "-";
const STDIN_LABEL: &str = "<stdin>";
const DIFF_CONTEXT: usize = 3;

#[derive(Parser)]
#[command(name = "phew")]
#[command(about = "Fast HTML + PHP formatter for Yii 2 view files")]
#[command(group(ArgGroup::new("mode").args(["write", "check", "diff", "tokens", "tree"])))]
struct Cli {
    #[arg(help = "Files or directories to format ('-' reads from stdin)")]
    paths: Vec<String>,

    #[arg(short, long, help = "Write result back to file")]
    write: bool,

    #[arg(long, help = "Exit non-zero if any file is not formatted; write nothing")]
    check: bool,

    #[arg(long, help = "Show what would change without writing")]
    diff: bool,

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

#[derive(Clone, Copy, Debug, PartialEq)]
enum Mode {
    Format,
    Write,
    Check,
    Diff,
    Tokens,
    Tree,
}

impl Mode {
    fn resolve(cli: &Cli) -> Self {
        if cli.write {
            Self::Write
        } else if cli.check {
            Self::Check
        } else if cli.diff {
            Self::Diff
        } else if cli.tokens {
            Self::Tokens
        } else if cli.tree {
            Self::Tree
        } else {
            Self::Format
        }
    }
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

fn format_content(content: &str, formatter: &Formatter) -> String {
    let tokens = lexer::tokenize(content);
    let nodes = ast::parse(tokens);
    formatter.format(&nodes)
}

fn dump_tokens(label: &str, content: &str) -> String {
    let tokens = lexer::tokenize(content);
    let mut out = format!("=== {label} ===\n");
    for token in &tokens {
        out.push_str(&format!("{token:?}\n"));
    }
    out
}

fn dump_tree(label: &str, content: &str) -> String {
    let tokens = lexer::tokenize(content);
    let nodes = ast::parse(tokens);
    let mut out = format!("=== {label} ===\n");
    render_tree(&nodes, 0, &mut out);
    out
}

fn diff_text(label: &str, content: &str, formatter: &Formatter) -> String {
    let formatted = format_content(content, formatter);
    if formatted == content {
        return String::new();
    }
    let diff = TextDiff::from_lines(content, &formatted);
    let mut unified = diff.unified_diff();
    unified
        .context_radius(DIFF_CONTEXT)
        .header(&format!("a/{label}"), &format!("b/{label}"));
    format!("{unified}")
}

fn load(path: &Path) -> Option<String> {
    match std::fs::read_to_string(path) {
        Ok(content) => Some(content),
        Err(e) => {
            eprintln!("Error reading {}: {e}", path.display());
            None
        }
    }
}

fn label(path: &Path) -> String {
    path.display().to_string()
}

fn print_pass<F>(files: &[PathBuf], render: F) -> ExitCode
where
    F: Fn(&Path, &str) -> String + Sync,
{
    let outputs: Vec<String> = files
        .par_iter()
        .map(|path| match load(path) {
            Some(content) => render(path, &content),
            None => String::new(),
        })
        .collect();

    let mut stdout = std::io::stdout().lock();
    for output in &outputs {
        let _ = stdout.write_all(output.as_bytes());
    }
    ExitCode::SUCCESS
}

fn write_pass(files: &[PathBuf], formatter: &Formatter) -> ExitCode {
    files.par_iter().for_each(|path| {
        let Some(content) = load(path) else {
            return;
        };
        let formatted = format_content(&content, formatter);
        if formatted != content
            && let Err(e) = std::fs::write(path, &formatted)
        {
            eprintln!("Error writing {}: {e}", path.display());
        }
    });
    ExitCode::SUCCESS
}

fn check_pass(files: &[PathBuf], formatter: &Formatter) -> ExitCode {
    let unformatted: Vec<&PathBuf> = files
        .par_iter()
        .filter(|path| match load(path) {
            Some(content) => format_content(&content, formatter) != content,
            None => false,
        })
        .collect();

    if unformatted.is_empty() {
        return ExitCode::SUCCESS;
    }

    let mut stderr = std::io::stderr().lock();
    for path in &unformatted {
        let _ = writeln!(stderr, "не отформатирован: {}", path.display());
    }
    let _ = writeln!(stderr, "Требуют форматирования: {}", unformatted.len());
    ExitCode::FAILURE
}

fn run_stdin(mode: Mode, formatter: &Formatter) -> anyhow::Result<ExitCode> {
    let mut content = String::new();
    std::io::stdin().read_to_string(&mut content)?;

    let exit = match mode {
        Mode::Format | Mode::Write => {
            print!("{}", format_content(&content, formatter));
            ExitCode::SUCCESS
        }
        Mode::Tokens => {
            print!("{}", dump_tokens(STDIN_LABEL, &content));
            ExitCode::SUCCESS
        }
        Mode::Tree => {
            print!("{}", dump_tree(STDIN_LABEL, &content));
            ExitCode::SUCCESS
        }
        Mode::Diff => {
            print!("{}", diff_text(STDIN_LABEL, &content, formatter));
            ExitCode::SUCCESS
        }
        Mode::Check => {
            if format_content(&content, formatter) == content {
                ExitCode::SUCCESS
            } else {
                eprintln!("stdin не отформатирован");
                ExitCode::FAILURE
            }
        }
    };
    Ok(exit)
}

fn is_stdin(paths: &[String]) -> bool {
    paths.len() == 1 && paths[0] == STDIN_MARKER
}

fn run() -> anyhow::Result<ExitCode> {
    let cli = Cli::parse();

    if cli.init {
        run_init()?;
        return Ok(ExitCode::SUCCESS);
    }

    if cli.paths.is_empty() {
        println!("phew v{}", env!("CARGO_PKG_VERSION"));
        return Ok(ExitCode::SUCCESS);
    }

    let config = resolve_config(&cli)?;
    let formatter = Formatter::new(&config);
    let mode = Mode::resolve(&cli);

    if is_stdin(&cli.paths) {
        return run_stdin(mode, &formatter);
    }

    let files = walker::collect_files(&cli.paths);
    let exit = match mode {
        Mode::Format => print_pass(&files, |_, content| format_content(content, &formatter)),
        Mode::Tokens => print_pass(&files, |path, content| dump_tokens(&label(path), content)),
        Mode::Tree => print_pass(&files, |path, content| dump_tree(&label(path), content)),
        Mode::Diff => print_pass(&files, |path, content| diff_text(&label(path), content, &formatter)),
        Mode::Write => write_pass(&files, &formatter),
        Mode::Check => check_pass(&files, &formatter),
    };
    Ok(exit)
}

fn main() -> ExitCode {
    if std::env::var_os("RUST_BACKTRACE").is_none() {
        std::panic::set_hook(Box::new(|_| {}));
    }

    match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{e:?}");
            ExitCode::FAILURE
        }
    }
}
