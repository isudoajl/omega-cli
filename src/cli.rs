use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "omg",
    version,
    about = "OMEGA \u{03A9} \u{2014} Deploy multi-agent workflows to any project"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Deploy OMEGA to the current project
    Init {
        /// Install extensions (comma-separated or "all")
        #[arg(long = "ext")]
        extensions: Option<String>,
        /// Skip SQLite initialization
        #[arg(long)]
        no_db: bool,
        /// Show all file statuses including unchanged
        #[arg(long)]
        verbose: bool,
        /// Show what would be deployed without writing
        #[arg(long)]
        dry_run: bool,
        /// Overwrite even if checksums match
        #[arg(long)]
        force: bool,
    },
    /// Update OMEGA in the current project
    Update {
        /// Update/add extensions (comma-separated or "all")
        #[arg(long = "ext")]
        extensions: Option<String>,
        /// Skip SQLite migration
        #[arg(long)]
        no_db: bool,
        /// Show all file statuses including unchanged
        #[arg(long)]
        verbose: bool,
        /// Show what would change
        #[arg(long)]
        dry_run: bool,
    },
    /// Check OMEGA installation health
    Doctor,
    /// Update the omg binary itself
    SelfUpdate {
        /// Only check if update is available
        #[arg(long)]
        check: bool,
    },
    /// List available extensions
    ListExt,
    /// Show version information
    Version {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Generate shell completions
    Completions {
        /// Shell: bash, zsh, fish
        shell: String,
    },
}
