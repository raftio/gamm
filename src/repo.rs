/*
 * repo.rs
 * Repo storage for tracking which git config profile owns each repository.
 *
 * - repo_name: friendly name for the repo
 * - url: remote URL (used for lookup)
 * - commit_by: references the config name in ConfigStore (e.g., "work", "personal")
 */

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::PathBuf;

/// A repository entry linking a remote URL to a config profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repo {
    pub repo_name: String,
    pub url: String,
    pub commit_by: String,
}

/// Store for managing repository ownership mappings
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RepoStore {
    /// Maps remote URL -> Repo
    repos: HashMap<String, Repo>,
}

impl RepoStore {
    /// Get the config directory path (~/.config/gam or platform equivalent)
    pub fn config_dir() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("gamm"))
    }

    /// Get the repos file path (~/.config/gam/repos.json)
    pub fn repos_path() -> Option<PathBuf> {
        Self::config_dir().map(|p| p.join("repos.json"))
    }

    /// Create a new empty store
    pub fn new() -> Self {
        Self {
            repos: HashMap::new(),
        }
    }

    /// Load store from the default repos file, or create new if it doesn't exist
    pub fn load() -> io::Result<Self> {
        let path = Self::repos_path()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Could not find config directory"))?;

        if path.exists() {
            let contents = fs::read_to_string(&path)?;
            serde_json::from_str(&contents)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
        } else {
            Ok(Self::new())
        }
    }

    /// Save store to the default repos file
    pub fn save(&self) -> io::Result<()> {
        let dir = Self::config_dir()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Could not find config directory"))?;
        let path = Self::repos_path().unwrap();

        // Create directory if it doesn't exist
        fs::create_dir_all(&dir)?;

        let contents = serde_json::to_string_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        fs::write(&path, contents)
    }

    /// Add a new repo to the store
    /// The URL is used as the key for lookups
    pub fn add(&mut self, repo: Repo) {
        self.repos.insert(repo.url.clone(), repo);
    }

    /// Add a new repo by individual fields
    pub fn add_repo(&mut self, repo_name: impl Into<String>, url: impl Into<String>, commit_by: impl Into<String>) {
        let url = url.into();
        self.repos.insert(
            url.clone(),
            Repo {
                repo_name: repo_name.into(),
                url,
                commit_by: commit_by.into(),
            },
        );
    }

    /// Look up who owns the repo by remote URL
    /// Returns the commit_by (config profile name) if found
    pub fn lookup_owner_by_url(&self, url: &str) -> Option<&str> {
        self.repos.get(url).map(|r| r.commit_by.as_str())
    }

    /// Get a repo by its remote URL
    pub fn get_by_url(&self, url: &str) -> Option<&Repo> {
        self.repos.get(url)
    }

    /// Remove a repo by its URL
    pub fn remove_by_url(&mut self, url: &str) -> Option<Repo> {
        self.repos.remove(url)
    }

    /// List all repo URLs
    pub fn list_urls(&self) -> impl Iterator<Item = &String> {
        self.repos.keys()
    }

    /// Iterate over all repos
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Repo)> {
        self.repos.iter()
    }

    /// Find all repos owned by a specific config profile
    pub fn find_by_owner(&self, commit_by: &str) -> Vec<&Repo> {
        self.repos
            .values()
            .filter(|r| r.commit_by == commit_by)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_repo() -> Repo {
        Repo {
            repo_name: "gam".into(),
            url: "git@github.com:9bany/gam.git".into(),
            commit_by: "personal".into(),
        }
    }

    #[test]
    fn test_new_store_is_empty() {
        let store = RepoStore::new();
        assert_eq!(store.list_urls().count(), 0);
    }

    #[test]
    fn test_add_and_get_repo() {
        let mut store = RepoStore::new();
        store.add(sample_repo());

        let repo = store.get_by_url("git@github.com:9bany/gam.git");
        assert!(repo.is_some());

        let repo = repo.unwrap();
        assert_eq!(repo.repo_name, "gam");
        assert_eq!(repo.commit_by, "personal");
    }

    #[test]
    fn test_add_repo_by_fields() {
        let mut store = RepoStore::new();
        store.add_repo("my-project", "git@github.com:user/project.git", "work");

        let repo = store.get_by_url("git@github.com:user/project.git");
        assert!(repo.is_some());
        assert_eq!(repo.unwrap().commit_by, "work");
    }

    #[test]
    fn test_lookup_owner_by_url() {
        let mut store = RepoStore::new();
        store.add(sample_repo());

        let owner = store.lookup_owner_by_url("git@github.com:9bany/gam.git");
        assert_eq!(owner, Some("personal"));

        let owner = store.lookup_owner_by_url("nonexistent");
        assert!(owner.is_none());
    }

    #[test]
    fn test_remove_repo() {
        let mut store = RepoStore::new();
        store.add(sample_repo());

        let removed = store.remove_by_url("git@github.com:9bany/gam.git");
        assert!(removed.is_some());
        assert!(store.get_by_url("git@github.com:9bany/gam.git").is_none());
    }

    #[test]
    fn test_find_by_owner() {
        let mut store = RepoStore::new();
        store.add_repo("project1", "url1", "work");
        store.add_repo("project2", "url2", "work");
        store.add_repo("project3", "url3", "personal");

        let work_repos = store.find_by_owner("work");
        assert_eq!(work_repos.len(), 2);

        let personal_repos = store.find_by_owner("personal");
        assert_eq!(personal_repos.len(), 1);
    }

    #[test]
    fn test_serialize_deserialize() {
        let mut store = RepoStore::new();
        store.add(sample_repo());

        let json = serde_json::to_string(&store).unwrap();
        let restored: RepoStore = serde_json::from_str(&json).unwrap();

        let repo = restored.get_by_url("git@github.com:9bany/gam.git").unwrap();
        assert_eq!(repo.repo_name, "gam");
        assert_eq!(repo.commit_by, "personal");
    }

    #[test]
    fn test_repos_path_exists() {
        let path = RepoStore::repos_path();
        assert!(path.is_some());
        assert!(path.unwrap().ends_with("repos.json"));
    }
}
