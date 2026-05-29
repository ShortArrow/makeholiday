use clap::Parser;

use makeholiday::application::use_cases;
use makeholiday::cli::{Cli, Commands};
use makeholiday::icons;
use makeholiday::infrastructure::FileCalendarRepository;

fn main() {
    let cli = Cli::parse();
    let repo = FileCalendarRepository::new(cli.file.clone());
    let result = match cli.command {
        Commands::Init => use_cases::init(&repo),
        Commands::Add {
            summary,
            start,
            end,
            busystatus,
            class,
            category,
            icon,
        } => use_cases::add(
            &repo,
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
            use_cases::list(&repo, &keys, desc, json).map(|output| {
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
            use_cases::remove(&repo, summary.as_deref(), target.as_deref())
        }
    };
    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
