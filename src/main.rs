use clap::Parser;

#[derive(Parser)]
#[command(name = "phrust")]
#[command(about = "Fast HTML + PHP formatter for Yii 2 view files")]
struct Cli {
    #[arg(help = "Files or directories to format")]
    paths: Vec<String>,

    #[arg(long, help = "Show tokens instead of formatting")]
    tokens: bool,
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

        if cli.tokens {
            let tokens = phrust::parser::lexer::tokenize(&content);
            println!("=== {path} ===");
            for token in &tokens {
                println!("{token:?}");
            }
        } else {
            println!("Would format: {path}");
        }
    }
}
