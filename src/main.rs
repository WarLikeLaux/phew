use clap::Parser;
use phrust::parser::{ast, lexer};

#[derive(Parser)]
#[command(name = "phrust")]
#[command(about = "Fast HTML + PHP formatter for Yii 2 view files")]
struct Cli {
    #[arg(help = "Files or directories to format")]
    paths: Vec<String>,

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
        }
    }
}

fn main() {
    let cli = Cli::parse();

    if cli.paths.is_empty() {
        println!("phrust v{}", env!("CARGO_PKG_VERSION"));
        return;
    }

    for path in &cli.paths {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error reading {path}: {e}");
                continue;
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
            print!("{}", phrust::formatter::engine::format(&nodes));
        }
    }
}
