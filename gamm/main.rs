mod command;
mod repo;
mod store;

use clap::{Parser, Subcommand};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(name = "gamm")]
#[command(about = "Git Account Manager - Manage multiple git configurations", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Display version information
    Version,
    /// Initialize gamm and install git hooks
    Init,
    /// Remove gamm git hooks
    Cleanup,
    /// Pre-commit hook: apply git config based on repository URL
    PreCommit {
        /// Remote repository URL
        #[arg(long)]
        repo: String,
    },
    /// Manage repository configurations
    Repo {
        #[command(subcommand)]
        action: RepoCommands,
    },
    /// Manage profile configurations
    Profile {
        #[command(subcommand)]
        action: ProfileCommands,
    },
}

#[derive(Subcommand)]
enum RepoCommands {
    /// List all configured repositories
    List,
    /// Delete a repository configuration
    #[command(alias = "rm")]
    Delete {
        /// Repository URL or name to delete (interactive if not provided)
        name: Option<String>,
    },
}

#[derive(Subcommand)]
enum ProfileCommands {
    /// List all configured profiles
    List,
    /// Delete a profile configuration (also removes related repositories)
    #[command(alias = "rm")]
    Delete {
        /// Profile name to delete (interactive if not provided)
        name: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Version => {
            println!("gamm {VERSION}");
        }
        Commands::Init => {
            if let Err(e) = command::init() {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Cleanup => {
            if let Err(e) = command::cleanup() {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::PreCommit { repo } => {
            if let Err(e) = command::pre_commit(&repo) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Repo { action } => match action {
            RepoCommands::List => {
                if let Err(e) = command::repo_list() {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
            RepoCommands::Delete { name } => {
                if let Err(e) = command::repo_delete(name) {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        },
        Commands::Profile { action } => match action {
            ProfileCommands::List => {
                if let Err(e) = command::profile_list() {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
            ProfileCommands::Delete { name } => {
                if let Err(e) = command::profile_delete(name) {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        },
    }
}
