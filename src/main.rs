mod cli;
mod commands;
mod ics;

use clap::Parser;
use cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();
    let file = &cli.file;
    let result = match cli.command {
        Commands::Init => commands::init(file),
        Commands::Add {
            summary,
            start,
            end,
        } => commands::add(file, summary.as_deref(), start, end),
        Commands::List { sort, desc } => {
            let keys: Vec<_> = sort.iter().map(|s| s.to_sort_key()).collect();
            commands::list(file, &keys, desc).map(|output| {
                if !output.is_empty() {
                    println!("{output}");
                }
            })
        }
        Commands::Remove { target, summary } => {
            commands::remove(file, summary.as_deref(), target.as_deref())
        }
    };
    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
