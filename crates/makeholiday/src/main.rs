use clap::Parser;

use makeholiday::cli::{Cli, Commands};
use makeholiday::{commands, icons};

fn main() {
    let cli = Cli::parse();
    let file = &cli.file;
    let result = match cli.command {
        Commands::Init => commands::init(file),
        Commands::Add {
            summary,
            start,
            end,
            busystatus,
            class,
            category,
            icon,
        } => commands::add(
            file,
            summary.as_deref(),
            start,
            end,
            busystatus.to_busystatus(),
            class.map(|c| c.to_event_class()),
            category,
            icon,
        ),
        Commands::List { sort, desc, json } => {
            let keys: Vec<_> = sort.iter().map(|s| s.to_sort_key()).collect();
            commands::list(file, &keys, desc, json).map(|output| {
                if !output.is_empty() {
                    println!("{output}");
                }
            })
        }
        Commands::Icons => {
            println!("{}", icons::format_icons_list());
            Ok(())
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
