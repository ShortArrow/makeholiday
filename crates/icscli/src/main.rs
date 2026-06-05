use std::io::IsTerminal;

use clap::Parser;

use icscli::application::use_cases::{self, RunContext};
use icscli::icons;
use icscli::infrastructure::FileCalendarRepository;
use icscli::presentation::{Cli, Commands};

fn main() {
    let cli = Cli::parse();
    let repo = FileCalendarRepository::new(cli.file.clone());
    let allow_prompts = if cli.no_interactive {
        false
    } else if cli.interactive {
        true
    } else {
        std::io::stdin().is_terminal()
    };
    let ctx = RunContext {
        quiet: cli.quiet,
        allow_prompts,
    };
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
            ctx,
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
        Commands::Edit {
            index,
            summary,
            start,
            end,
            busystatus,
            class,
            category,
            category_clear,
            icon,
            icon_clear,
        } => {
            let patch = use_cases::EditPatch {
                summary,
                start,
                end,
                busystatus: busystatus.map(|b| b.to_busystatus()),
                class: class.map(|c| c.to_event_class()),
                categories: if category.is_empty() {
                    None
                } else {
                    Some(category)
                },
                clear_categories: category_clear,
                icon,
                clear_icon: icon_clear,
            };
            use_cases::edit(&repo, ctx, index, patch)
        }
        Commands::Icons => {
            println!("{}", icons::format_icons_list());
            Ok(())
        }
        Commands::Remove { target, summary } => {
            use_cases::remove(&repo, ctx, summary.as_deref(), target.as_deref())
        }
        Commands::Split { from, to, uid, out } => {
            let out_repo = FileCalendarRepository::new(out);
            use_cases::split(&repo, &out_repo, ctx, from, to, &uid)
        }
    };
    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
