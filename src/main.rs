use clap::Parser;

#[derive(Parser)]
#[command(name = "phrust")]
#[command(about = "Fast HTML + PHP formatter for Yii 2 view files")]
struct Cli {
    #[arg(help = "Files or directories to format")]
    paths: Vec<String>,
}

fn main() {
    let cli = Cli::parse();

    if cli.paths.is_empty() {
        println!("phrust v{}", env!("CARGO_PKG_VERSION"));
        return;
    }

    for path in &cli.paths {
        println!("Would format: {path}");
    }
}
