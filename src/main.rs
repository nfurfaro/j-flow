use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;
mod config;
mod jj;
mod ui;

use config::Config;

#[derive(Parser)]
#[command(name = "jf")]
#[command(version, about = "Beautiful workflow tool for Jujutsu VCS", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize jflow in the current repository
    Init {
        /// Skip interactive prompts and use defaults
        #[arg(short, long)]
        defaults: bool,

        /// Create a GitHub repository (uses gh CLI)
        #[arg(long)]
        github: bool,

        /// Force creating local .jflow.toml even if global config exists
        #[arg(short, long)]
        local: bool,
    },

    /// Show your stack with PR status
    Status,

    /// Push changes to GitHub, creating or updating PRs
    Push {
        /// Revset of changes to push (default: entire stack)
        #[arg(short, long)]
        revision: Option<String>,

        /// Bookmark name for the change (required for new PRs)
        #[arg(short, long)]
        bookmark: Option<String>,

        /// Force squash-style push (override config)
        #[arg(long)]
        squash: bool,

        /// Force append-style push (override config)
        #[arg(long)]
        append: bool,

        /// Dry run - show what would be done
        #[arg(short = 'n', long)]
        dry_run: bool,
    },

    /// Clean up after PRs are merged
    Land {
        /// Specific bookmark to land (default: auto-detect merged)
        bookmark: Option<String>,

        /// Dry run - show what would be done
        #[arg(short = 'n', long)]
        dry_run: bool,
    },

    /// Pull from remote and rebase your stack
    Pull {
        /// Remote to pull from
        #[arg(short, long)]
        remote: Option<String>,
    },

    /// Reorder changes in the stack
    Reorder {
        /// Changes to reorder (in desired order, e.g., "abc def ghi")
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        changes: Vec<String>,

        /// Invert (reverse) the stack order
        #[arg(short, long)]
        invert: bool,

        /// Starting change for --invert (default: entire stack)
        #[arg(short, long)]
        from: Option<String>,
    },

    /// Sync work-in-progress between machines
    Wip {
        /// Subcommand: push, pull, clean (or none for status)
        subcommand: Option<String>,

        /// Force overwrite (push) or delete without PR check (clean)
        #[arg(short, long)]
        force: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Init { defaults, github, local }) => {
            // Init doesn't need existing config
            commands::init::run(defaults, github, local)?
        }
        None => {
            // No command = run status
            let config = Config::load_or_default()?;
            commands::status::run(&config)?
        }
        Some(cmd) => {
            // Other commands load config normally
            let config = Config::load_or_default()?;

            match cmd {
                Commands::Init { .. } => unreachable!(),
                Commands::Status => commands::status::run(&config)?,
                Commands::Push {
                    revision,
                    bookmark,
                    squash,
                    append,
                    dry_run,
                } => {
                    commands::push::run(
                        &config,
                        revision.as_deref(),
                        bookmark.as_deref(),
                        squash,
                        append,
                        dry_run,
                    )?
                }
                Commands::Land { bookmark, dry_run } => {
                    commands::land::run(&config, bookmark.as_deref(), dry_run)?
                }
                Commands::Pull { remote } => {
                    commands::pull::run(&config, remote.as_deref())?
                }
                Commands::Reorder { changes, invert, from } => {
                    commands::reorder::run(&config, changes, invert, from.as_deref())?
                }
                Commands::Wip { subcommand, force } => {
                    commands::wip::run(&config, subcommand.as_deref(), force)?
                }
            }
        }
    }

    Ok(())
}
