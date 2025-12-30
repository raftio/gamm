use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;

use crate::repo::{Repo, RepoStore};
use crate::store::{self, ConfigStore};

const GAM_MARKER_START: &str = "# >>> gamm";
const GAM_MARKER_END: &str = "# <<< gamm";

const GAM_HOOK_SECTION: &str = r#"# >>> gamm

REMOTE_URL=$(git remote get-url origin 2>/dev/null || true)
[ -z "$REMOTE_URL" ] && exit 0

echo "gamm: checking ..."
gamm pre-commit --repo "$REMOTE_URL"
# <<< gamm"#;

fn get_githooks_dir() -> PathBuf {
    dirs::home_dir()
        .expect("Could not find home directory")
        .join(".githooks")
}

/// Get the current git user.email from global config
fn get_current_git_email() -> Option<String> {
    let output = Command::new("git")
        .args(["config", "--global", "user.email"])
        .output()
        .ok()?;

    if output.status.success() {
        let email = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !email.is_empty() {
            return Some(email);
        }
    }

    None
}

/// Get the current git user.name from global config
fn get_current_git_name() -> Option<String> {
    let output = Command::new("git")
        .args(["config", "--global", "user.name"])
        .output()
        .ok()?;

    if output.status.success() {
        let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !name.is_empty() {
            return Some(name);
        }
    }

    None
}

/// Apply git config for the given owner
fn apply_git_config(owner: &str, config: &store::GitConfig, repo_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Applying config '{}' for {}", owner, repo_url);

    // Set user.name
    if !config.user.name.is_empty() {
        Command::new("git")
            .args(["config", "--global", "user.name", &config.user.name])
            .status()?;
        println!("  user.name = {}", config.user.name);
    }

    // Set user.email
    if !config.user.email.is_empty() {
        Command::new("git")
            .args(["config", "--global", "user.email", &config.user.email])
            .status()?;
        println!("  user.email = {}", config.user.email);
    }

    // Set commit.gpgsign
    Command::new("git")
        .args([
            "config",
            "--global",
            "commit.gpgsign",
            if config.commit.gpgsign { "true" } else { "false" },
        ])
        .status()?;
    println!("  commit.gpgsign = {}", config.commit.gpgsign);

    // Apply URL rewrites
    for url_config in &config.urls {
        Command::new("git")
            .args([
                "config",
                "--global",
                &format!("url.{}.insteadOf", url_config.pattern),
                &url_config.instead_of,
            ])
            .status()?;
        println!(
            "  url.{}.insteadOf = {}",
            url_config.pattern, url_config.instead_of
        );
    }

    Ok(())
}

/// Show interactive UI to add a new config profile
fn add_config_interactive(config_store: &mut ConfigStore) -> Result<String, Box<dyn std::error::Error>> {
    let theme = ColorfulTheme::default();

    println!();
    println!("┌─────────────────────────────────────────────────────────────┐");
    println!("│  Create a new git config profile                            │");
    println!("└─────────────────────────────────────────────────────────────┘");
    println!();

    // Ask for profile name
    let profile_name: String = Input::with_theme(&theme)
        .with_prompt("Profile name (e.g., 'work', 'personal')")
        .interact_text()?;

    // Ask for user.name
    let default_name = get_current_git_name().unwrap_or_default();
    let user_name: String = Input::with_theme(&theme)
        .with_prompt("user.name")
        .default(default_name)
        .interact_text()?;

    // Ask for user.email
    let default_email = get_current_git_email().unwrap_or_default();
    let user_email: String = Input::with_theme(&theme)
        .with_prompt("user.email")
        .default(default_email)
        .interact_text()?;

    // Ask for gpgsign
    let gpgsign = Confirm::with_theme(&theme)
        .with_prompt("Enable GPG signing for commits?")
        .default(false)
        .interact()?;

    // Create and save the config
    let git_config = store::GitConfig {
        user: store::UserConfig {
            name: user_name,
            email: user_email,
            signoff: None,
        },
        urls: vec![],
        commit: store::CommitConfig { gpgsign },
    };

    config_store.add(profile_name.clone(), git_config.clone());
    config_store.save()?;

    // Apply the config to git immediately
    println!();
    println!("Applying config '{}'...", profile_name);

    Command::new("git")
        .args(["config", "--global", "user.name", &git_config.user.name])
        .status()?;
    println!("  user.name = {}", git_config.user.name);

    Command::new("git")
        .args(["config", "--global", "user.email", &git_config.user.email])
        .status()?;
    println!("  user.email = {}", git_config.user.email);

    Command::new("git")
        .args([
            "config",
            "--global",
            "commit.gpgsign",
            if git_config.commit.gpgsign { "true" } else { "false" },
        ])
        .status()?;
    println!("  commit.gpgsign = {}", git_config.commit.gpgsign);

    println!();
    println!("✓ Config profile '{}' created and applied!", profile_name);

    Ok(profile_name)
}

/// Show interactive UI to add a new repo to gam configuration
fn add_repo_interactive(repo_url: &str, config_store: &mut ConfigStore) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let theme = ColorfulTheme::default();

    // Get list of available config profiles
    let mut profiles: Vec<String> = config_store.list().cloned().collect();

    if profiles.is_empty() {
        println!();
        println!("┌─────────────────────────────────────────────────────────────┐");
        println!("│  No git config profiles found!                              │");
        println!("└─────────────────────────────────────────────────────────────┘");
        println!();
        println!("  Let's create your first config profile.");

        let profile_name = add_config_interactive(config_store)?;
        profiles.push(profile_name);
    }

    println!();
    println!("┌─────────────────────────────────────────────────────────────┐");
    println!("│  New repository detected!                                   │");
    println!("└─────────────────────────────────────────────────────────────┘");
    println!();
    println!("  Repository: {}", repo_url);
    println!();

    // Ask if user wants to add this repo
    let add_repo = Confirm::with_theme(&theme)
        .with_prompt("Would you like to add this repository to gamm?")
        .default(true)
        .interact()?;

    if !add_repo {
        println!("Skipping repository setup.");
        return Ok(None);
    }

    // Ask for a friendly name for the repo
    let default_name = repo_url
        .rsplit('/')
        .next()
        .unwrap_or("repo")
        .trim_end_matches(".git");

    let repo_name: String = Input::with_theme(&theme)
        .with_prompt("Enter a name for this repository")
        .default(default_name.to_string())
        .interact_text()?;

    // Show selection for owner
    println!();
    println!("Select the git config profile (owner) for this repository:");
    println!();

    // Build display items with profile details
    let mut display_items: Vec<String> = profiles
        .iter()
        .map(|profile| {
            if let Some(config) = config_store.get(profile) {
                format!("{} - {} <{}>", profile, config.user.name, config.user.email)
            } else {
                profile.clone()
            }
        })
        .collect();

    // Add "Create new profile" option at the end
    display_items.push("+ Create new profile".to_string());

    let selection = Select::with_theme(&theme)
        .with_prompt("Choose owner")
        .items(&display_items)
        .default(0)
        .interact()?;

    // Check if user selected "Create new profile"
    let selected_owner = if selection == profiles.len() {
        // Create new profile
        let new_profile = add_config_interactive(config_store)?;
        new_profile
    } else {
        profiles[selection].clone()
    };

    // Save the repo to the store
    let mut repo_store = RepoStore::load()?;
    repo_store.add(Repo {
        repo_name,
        url: repo_url.to_string(),
        commit_by: selected_owner.clone(),
    });
    repo_store.save()?;

    println!();
    println!("✓ Repository added with owner '{}'", selected_owner);

    Ok(Some(selected_owner))
}

pub fn init() -> Result<(), Box<dyn std::error::Error>> {
    let githooks_dir = get_githooks_dir();
    let pre_commit_path = githooks_dir.join("pre-commit");

    // Create ~/.githooks directory if it doesn't exist
    if !githooks_dir.exists() {
        fs::create_dir_all(&githooks_dir)?;
        println!("Created directory: {}", githooks_dir.display());
    }

    let new_content = if pre_commit_path.exists() {
        let existing = fs::read_to_string(&pre_commit_path)?;
        
        // Check if gam section already exists
        if existing.contains(GAM_MARKER_START) {
            println!("gamm hook already installed in: {}", pre_commit_path.display());
            return Ok(());
        }
        
        // Append gam section to existing file
        format!("{}\n\n{}\n", existing.trim_end(), GAM_HOOK_SECTION)
    } else {
        // Create new file with shebang
        format!("#!/bin/sh\nset -e\n\n{}\n", GAM_HOOK_SECTION)
    };

    fs::write(&pre_commit_path, new_content)?;

    // Make the script executable (chmod +x)
    let mut perms = fs::metadata(&pre_commit_path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&pre_commit_path, perms)?;

    println!("Installed pre-commit hook: {}", pre_commit_path.display());
    println!();
    println!("To enable the hook globally, run:");
    println!("  git config --global core.hooksPath ~/.githooks");

    if let Some(config_dir) = store::ConfigStore::config_dir() {
        println!();
        println!("Config storage: {}", config_dir.display());
    }

    Ok(())
}

pub fn pre_commit(repo_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Load the stores
    let repo_store = RepoStore::load()?;
    let mut config_store = ConfigStore::load()?;

    // Check if the repo exists in the store
    if let Some(owner) = repo_store.lookup_owner_by_url(repo_url) {
        // Repo exists - verify owner matches current git config
        let config = match config_store.get(owner) {
            Some(config) => config,
            None => {
                eprintln!("Warning: repo mapped to config '{}' but config not found", owner);
                return Ok(());
            }
        };

        // Get current git config
        let current_email = get_current_git_email();
        let current_name = get_current_git_name();

        // Check if current config matches the expected owner config
        let email_matches = current_email.as_ref().map_or(false, |e| e == &config.user.email);
        let name_matches = current_name.as_ref().map_or(false, |n| n == &config.user.name);

        if email_matches && name_matches {
            // Config already matches, nothing to do
            println!("✓ Git config already set for '{}' ({})", owner, config.user.email);
            return Ok(());
        }

        // Config doesn't match - show what's different and apply
        if !email_matches || !name_matches {
            println!("┌─────────────────────────────────────────────────────────────┐");
            println!("│  Git config mismatch detected                               │");
            println!("└─────────────────────────────────────────────────────────────┘");
            println!();
            println!("  Repository: {}", repo_url);
            println!("  Expected owner: {} ({})", owner, config.user.email);
            println!();

            if let Some(ref email) = current_email {
                if !email_matches {
                    println!("  Current email: {} (will change to: {})", email, config.user.email);
                }
            } else {
                println!("  Current email: <not set> (will set to: {})", config.user.email);
            }

            if let Some(ref name) = current_name {
                if !name_matches {
                    println!("  Current name: {} (will change to: {})", name, config.user.name);
                }
            } else {
                println!("  Current name: <not set> (will set to: {})", config.user.name);
            }

            println!();
        }

        // Apply the config
        apply_git_config(owner, config, repo_url)?;
        
        // Abort the commit so user can retry with correct config
        println!();
        println!("⚠ Config updated. Please run your commit command again.");
        std::process::exit(1);
    } else {
        // Repo doesn't exist - show interactive UI to add it
        if let Some(owner) = add_repo_interactive(repo_url, &mut config_store)? {
            // Apply the config for the newly added repo
            if let Some(config) = config_store.get(&owner) {
                apply_git_config(&owner, config, repo_url)?;
            }
            
            // Abort the commit so user can retry with correct config
            println!();
            println!("⚠ Config applied. Please run your commit command again.");
            std::process::exit(1);
        }
    }

    Ok(())
}

/// List all configured repositories
pub fn repo_list() -> Result<(), Box<dyn std::error::Error>> {
    let repo_store = RepoStore::load()?;
    let config_store = ConfigStore::load()?;

    let repos: Vec<_> = repo_store.iter().collect();

    if repos.is_empty() {
        println!("No repositories configured.");
        println!();
        println!("Repositories are automatically added when you commit to a new repo.");
        return Ok(());
    }

    println!();
    println!("┌─────────────────────────────────────────────────────────────┐");
    println!("│  Configured Repositories                                    │");
    println!("└─────────────────────────────────────────────────────────────┘");
    println!();

    for (_url, repo) in repos {
        let owner_info = config_store
            .get(&repo.commit_by)
            .map(|c| format!("{} <{}>", c.user.name, c.user.email))
            .unwrap_or_else(|| "(config not found)".to_string());

        println!("  {} ", repo.repo_name);
        println!("    URL:   {}", repo.url);
        println!("    Owner: {} ({})", repo.commit_by, owner_info);
        println!();
    }

    Ok(())
}

/// Delete a repository configuration
pub fn repo_delete(name: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let mut repo_store = RepoStore::load()?;
    let repos: Vec<_> = repo_store.iter().map(|(url, repo)| (url.clone(), repo.clone())).collect();

    if repos.is_empty() {
        println!("No repositories configured.");
        return Ok(());
    }

    let to_delete = if let Some(ref name) = name {
        // Find by name or URL
        repos.iter()
            .find(|(url, repo)| repo.repo_name == *name || url == name)
            .map(|(url, _)| url.clone())
    } else {
        // Interactive selection
        let theme = ColorfulTheme::default();

        println!();
        println!("Select a repository to delete:");
        println!();

        let items: Vec<String> = repos
            .iter()
            .map(|(_, repo)| format!("{} ({})", repo.repo_name, repo.url))
            .collect();

        let selection = Select::with_theme(&theme)
            .with_prompt("Choose repository")
            .items(&items)
            .default(0)
            .interact()?;

        Some(repos[selection].0.clone())
    };

    match to_delete {
        Some(url) => {
            let repo = repo_store.remove_by_url(&url);
            if let Some(repo) = repo {
                repo_store.save()?;
                println!("✓ Deleted repository '{}'", repo.repo_name);
            }
        }
        None => {
            if let Some(name) = name {
                println!("Repository '{}' not found.", name);
            }
        }
    }

    Ok(())
}

/// List all configured profiles
pub fn profile_list() -> Result<(), Box<dyn std::error::Error>> {
    let config_store = ConfigStore::load()?;

    let profiles: Vec<_> = config_store.iter().collect();

    if profiles.is_empty() {
        println!("No profiles configured.");
        println!();
        println!("Create a profile by running: gamm pre-commit --repo <url>");
        return Ok(());
    }

    println!();
    println!("┌─────────────────────────────────────────────────────────────┐");
    println!("│  Configured Profiles                                        │");
    println!("└─────────────────────────────────────────────────────────────┘");
    println!();

    for (name, config) in profiles {
        println!("  {} ", name);
        println!("    Name:     {}", config.user.name);
        println!("    Email:    {}", config.user.email);
        println!("    GPG Sign: {}", if config.commit.gpgsign { "yes" } else { "no" });
        if !config.urls.is_empty() {
            println!("    URL Rewrites:");
            for url in &config.urls {
                println!("      {} -> {}", url.instead_of, url.pattern);
            }
        }
        println!();
    }

    Ok(())
}

/// Delete a profile configuration (also removes related repositories)
pub fn profile_delete(name: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let mut config_store = ConfigStore::load()?;
    let mut repo_store = RepoStore::load()?;

    let profiles: Vec<_> = config_store.iter().map(|(n, _)| n.clone()).collect();

    if profiles.is_empty() {
        println!("No profiles configured.");
        return Ok(());
    }

    let to_delete = if let Some(ref name) = name {
        // Find by name
        if profiles.contains(name) {
            Some(name.clone())
        } else {
            None
        }
    } else {
        // Interactive selection
        let theme = ColorfulTheme::default();

        println!();
        println!("Select a profile to delete:");
        println!();

        let items: Vec<String> = config_store
            .iter()
            .map(|(name, config)| format!("{} - {} <{}>", name, config.user.name, config.user.email))
            .collect();

        let profile_names: Vec<_> = config_store.iter().map(|(n, _)| n.clone()).collect();

        let selection = Select::with_theme(&theme)
            .with_prompt("Choose profile")
            .items(&items)
            .default(0)
            .interact()?;

        Some(profile_names[selection].clone())
    };

    match to_delete {
        Some(profile_name) => {
            // Find and remove all repos associated with this profile
            let related_repos: Vec<String> = repo_store
                .find_by_owner(&profile_name)
                .iter()
                .map(|r| r.url.clone())
                .collect();

            let removed_repos_count = related_repos.len();

            // Remove all related repos
            for url in related_repos {
                repo_store.remove_by_url(&url);
            }

            // Remove the profile
            let removed = config_store.remove(&profile_name);
            if removed.is_some() {
                config_store.save()?;
                repo_store.save()?;

                println!("✓ Deleted profile '{}'", profile_name);
                if removed_repos_count > 0 {
                    println!("✓ Cleaned up {} related repository configuration(s)", removed_repos_count);
                }
            }
        }
        None => {
            if let Some(name) = name {
                println!("Profile '{}' not found.", name);
            }
        }
    }

    Ok(())
}

pub fn cleanup() -> Result<(), Box<dyn std::error::Error>> {
    let githooks_dir = get_githooks_dir();
    let pre_commit_path = githooks_dir.join("pre-commit");

    if !pre_commit_path.exists() {
        println!("No pre-commit hook found at: {}", pre_commit_path.display());
        return Ok(());
    }

    let content = fs::read_to_string(&pre_commit_path)?;
    
    if !content.contains(GAM_MARKER_START) {
        println!("No gamm config found in: {}", pre_commit_path.display());
        return Ok(());
    }

    // Remove the gam section (including markers and surrounding newlines)
    let mut new_content = String::new();
    let mut in_gam_section = false;
    
    for line in content.lines() {
        if line.trim() == GAM_MARKER_START {
            in_gam_section = true;
            continue;
        }
        if line.trim() == GAM_MARKER_END {
            in_gam_section = false;
            continue;
        }
        if !in_gam_section {
            new_content.push_str(line);
            new_content.push('\n');
        }
    }

    // Clean up extra blank lines
    let new_content = new_content.trim_end().to_string();
    
    // Check if remaining content is just shebang/empty
    let is_empty = new_content.lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with("#!") && !l.starts_with("set -e"))
        .count() == 0;

    if is_empty {
        // Remove the file entirely if only gam was in it
        fs::remove_file(&pre_commit_path)?;
        println!("Removed pre-commit hook: {}", pre_commit_path.display());
        
        // Remove the directory if it's empty
        if githooks_dir.read_dir()?.next().is_none() {
            fs::remove_dir(&githooks_dir)?;
            println!("Removed empty directory: {}", githooks_dir.display());
        }
    } else {
        // Write back the file without gam section
        fs::write(&pre_commit_path, format!("{}\n", new_content))?;
        println!("Removed gamm config from: {}", pre_commit_path.display());
    }

    // Clean up config files
    if let Some(config_path) = store::ConfigStore::config_path() {
        if config_path.exists() {
            fs::remove_file(&config_path)?;
            println!("Removed config: {}", config_path.display());
        }
    }

    if let Some(repos_path) = crate::repo::RepoStore::repos_path() {
        if repos_path.exists() {
            fs::remove_file(&repos_path)?;
            println!("Removed repos: {}", repos_path.display());
        }
    }

    // Remove config directory if empty
    if let Some(config_dir) = store::ConfigStore::config_dir() {
        if config_dir.exists() && config_dir.read_dir()?.next().is_none() {
            fs::remove_dir(&config_dir)?;
            println!("Removed empty directory: {}", config_dir.display());
        }
    }

    Ok(())
}

