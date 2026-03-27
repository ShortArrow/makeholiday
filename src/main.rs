mod cli;
mod commands;
mod ics;

use clap::Parser;
use cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Init { file } => commands::init(&file),
        Commands::Add {
            file,
            summary,
            start,
            end,
        } => commands::add(&file, &summary, start, end),
        Commands::List { file } => commands::list(&file).map(|output| {
            if !output.is_empty() {
                println!("{output}");
            }
        }),
    };
    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
