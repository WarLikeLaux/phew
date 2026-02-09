use clap::Parser;
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

fn process_file(path: &str, cli: &Cli) {
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
        let formatted = phew::formatter::engine::format(&nodes);
        if cli.write {
            if let Err(e) = std::fs::write(path, &formatted) {
                eprintln!("Error writing {path}: {e}");
            }
        } else {
            print!("{formatted}");
        }
    }
}

fn main() {
    let cli = Cli::parse();

    if cli.paths.is_empty() {
        println!("phew v{}", env!("CARGO_PKG_VERSION"));
        return;
    }

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
        process_file(path, &cli);
    }
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
