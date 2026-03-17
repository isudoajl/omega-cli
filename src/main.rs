mod assets;
mod claude_md;
mod cli;
mod db;
mod deploy;
mod doctor;
mod self_update;
mod settings;
mod version;

use std::process;

use clap::Parser;
use console::style;

use cli::{Cli, Commands};
use deploy::{DeployEngine, DeployOptions};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init {
            extensions,
            no_db,
            verbose,
            dry_run,
            force,
        } => {
            let target = std::env::current_dir().unwrap_or_else(|e| {
                eprintln!("Error: cannot determine current directory: {}", e);
                process::exit(1);
            });

            let ext_list = parse_extensions(extensions);
            let opts = DeployOptions {
                extensions: ext_list,
                skip_db: no_db,
                verbose,
                dry_run,
                force,
            };

            println!();
            println!(
                "{}",
                style("  OMEGA \u{03A9} \u{2014} Deploying to project...").bold()
            );
            println!();

            let engine = DeployEngine::new(target, opts);
            match engine.deploy() {
                Ok(report) => {
                    report.print_summary(verbose);
                    if !report.errors.is_empty() {
                        process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("{} {}", style("Error:").red().bold(), e);
                    process::exit(1);
                }
            }
        }
        Commands::Update {
            extensions,
            no_db,
            verbose,
            dry_run,
        } => {
            let target = std::env::current_dir().unwrap_or_else(|e| {
                eprintln!("Error: cannot determine current directory: {}", e);
                process::exit(1);
            });

            let ext_list = parse_extensions(extensions);
            let opts = DeployOptions {
                extensions: ext_list,
                skip_db: no_db,
                verbose,
                dry_run,
                force: false,
            };

            println!();
            println!(
                "{}",
                style("  OMEGA \u{03A9} \u{2014} Updating project...").bold()
            );
            println!();

            let engine = DeployEngine::new(target, opts);
            match engine.deploy() {
                Ok(report) => {
                    report.print_summary(verbose);
                    if !report.errors.is_empty() {
                        process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("{} {}", style("Error:").red().bold(), e);
                    process::exit(1);
                }
            }
        }
        Commands::Doctor => {
            let target = std::env::current_dir().unwrap_or_default();
            let report = doctor::run_diagnostics(&target);
            report.print();
            if matches!(report.overall, doctor::OverallHealth::Broken) {
                process::exit(1);
            }
        }
        Commands::SelfUpdate { check } => {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create async runtime");
            if let Err(e) = rt.block_on(self_update::run(check)) {
                eprintln!("  {} {}", style("Error:").red().bold(), e);
                process::exit(1);
            }
        }
        Commands::ListExt => {
            let exts = assets::extensions();
            if exts.is_empty() {
                println!("No extensions available.");
                return;
            }
            println!();
            println!("{}", style("Available extensions:").bold());
            println!();
            for ext in exts {
                println!(
                    "  {}  ({} agents, {} commands)",
                    style(ext.name).cyan().bold(),
                    ext.agents.len(),
                    ext.commands.len(),
                );
                if !ext.agents.is_empty() {
                    let names: Vec<&str> = ext
                        .agents
                        .iter()
                        .map(|a| a.name.trim_end_matches(".md"))
                        .collect();
                    println!("    Agents:   {}", names.join(", "));
                }
                if !ext.commands.is_empty() {
                    let names: Vec<&str> = ext
                        .commands
                        .iter()
                        .map(|c| c.name.trim_end_matches(".md"))
                        .collect();
                    println!("    Commands: {}", names.join(", "));
                }
                println!();
            }
            println!(
                "Install with: {}",
                style("omg init --ext=<name1,name2|all>").cyan()
            );
            println!();
        }
        Commands::Version { json } => {
            version::print_version(json);
        }
        Commands::Completions { shell: _ } => {
            println!("Shell completions are not yet implemented.");
            println!("Coming in a future release.");
        }
    }
}

/// Parse the `--ext` flag value into a list of extension names.
fn parse_extensions(ext: Option<String>) -> Vec<String> {
    match ext {
        Some(s) if s == "all" => vec!["all".to_string()],
        Some(s) => s
            .split(',')
            .map(|e| e.trim().to_string())
            .filter(|e| !e.is_empty())
            .collect(),
        None => Vec::new(),
    }
}
