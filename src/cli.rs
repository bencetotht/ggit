use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "ggit", version, about = "Global Git tracking and management")]
pub struct Cli {
    #[arg(long, global = true, help = "Print extra command details")]
    pub verbose: bool,

    #[arg(long, global = true, help = "Suppress non-error output")]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(about = "Scan a directory tree and store discovered Git repositories")]
    Scan {
        #[arg(value_name = "PATH")]
        path: PathBuf,

        #[arg(long, value_name = "N", help = "Maximum traversal depth from PATH")]
        maxdepth: Option<usize>,

        #[arg(long, help = "Do not skip hidden directories except .git")]
        include_hidden: bool,

        #[arg(long, help = "Update already-known repositories while scanning")]
        force: bool,

        #[arg(
            long,
            help = "Show discovered repositories without changing the database"
        )]
        dry_run: bool,
    },

    #[command(about = "Run git pull --ff-only for tracked repositories")]
    Pull {
        #[arg(
            long,
            value_name = "NAME",
            help = "Case-insensitive repository name filter"
        )]
        filter: Option<String>,

        #[arg(
            long,
            value_name = "PATH",
            help = "Restrict to one tracked repository path"
        )]
        path: Option<PathBuf>,

        #[arg(long, help = "Show repositories that would be pulled")]
        dry_run: bool,

        #[arg(long, help = "Keep processing repositories after a failure")]
        continue_on_error: bool,
    },

    #[command(about = "Show Git status for tracked repositories")]
    Status {
        #[arg(
            long,
            value_name = "NAME",
            help = "Case-insensitive repository name filter"
        )]
        filter: Option<String>,

        #[arg(
            long,
            value_name = "PATH",
            help = "Restrict to one tracked repository path"
        )]
        path: Option<PathBuf>,

        #[arg(long, help = "Use compact table output")]
        short: bool,

        #[arg(long, help = "Accepted for future compatibility; status is local-only")]
        refresh_remotes: bool,
    },

    #[command(about = "List repositories stored in the registry")]
    List {
        #[arg(
            long,
            value_name = "NAME",
            help = "Case-insensitive repository name filter"
        )]
        filter: Option<String>,
    },

    #[command(about = "Remove repositories from the registry")]
    Remove {
        #[arg(
            long,
            value_name = "NAME",
            help = "Case-insensitive repository name filter"
        )]
        filter: Option<String>,

        #[arg(long, value_name = "PATH", help = "Remove one tracked repository path")]
        path: Option<PathBuf>,

        #[arg(long, help = "Confirm removal of multiple matches")]
        yes: bool,
    },

    #[command(about = "Clear all repositories from the registry")]
    Clear {
        #[arg(long, help = "Confirm clearing the registry")]
        yes: bool,
    },

    #[command(about = "Check ggit, Git, and registry health")]
    Doctor,
}
