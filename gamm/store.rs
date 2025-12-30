use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::PathBuf;

/// User configuration section
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserConfig {
    pub email: String,
    pub name: String,
    pub signoff: Option<String>,
}

/// URL rewrite rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlConfig {
    pub pattern: String,
    pub instead_of: String,
}

/// Commit configuration section
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommitConfig {
    pub gpgsign: bool,
}

/// A complete git configuration profile
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitConfig {
    pub user: UserConfig,
    pub urls: Vec<UrlConfig>,
    pub commit: CommitConfig,
}

/// Store for managing multiple git config profiles
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ConfigStore {
    #[serde(flatten)]
    configs: HashMap<String, GitConfig>,
}

impl ConfigStore {
    /// Get the config directory path (~/.config/gam or platform equivalent)
    pub fn config_dir() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("gamm"))
    }

    /// Get the config file path (~/.config/gam/config.json)
    pub fn config_path() -> Option<PathBuf> {
        Self::config_dir().map(|p| p.join("config.json"))
    }

    /// Create a new empty store
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
        }
    }

    /// Load store from the default config file, or create new if it doesn't exist
    pub fn load() -> io::Result<Self> {
        let path = Self::config_path()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Could not find config directory"))?;

        if path.exists() {
            let contents = fs::read_to_string(&path)?;
            serde_json::from_str(&contents)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
        } else {
            Ok(Self::new())
        }
    }

    /// Save store to the default config file
    pub fn save(&self) -> io::Result<()> {
        let dir = Self::config_dir()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Could not find config directory"))?;
        let path = Self::config_path().unwrap();

        // Create directory if it doesn't exist
        fs::create_dir_all(&dir)?;

        let contents = serde_json::to_string_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        fs::write(&path, contents)
    }

    pub fn add(&mut self, name: impl Into<String>, config: GitConfig) {
        self.configs.insert(name.into(), config);
    }

    pub fn get(&self, name: &str) -> Option<&GitConfig> {
        self.configs.get(name)
    }

    pub fn remove(&mut self, name: &str) -> Option<GitConfig> {
        self.configs.remove(name)
    }

    pub fn list(&self) -> impl Iterator<Item = &String> {
        self.configs.keys()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &GitConfig)> {
        self.configs.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn sample_config() -> GitConfig {
        GitConfig {
            user: UserConfig {
                email: "test@example.com".into(),
                name: "Test User".into(),
                signoff: Some("test".into()),
            },
            urls: vec![UrlConfig {
                pattern: "git@github.com:".into(),
                instead_of: "https://github.com/".into(),
            }],
            commit: CommitConfig { gpgsign: true },
        }
    }

    #[test]
    fn test_new_store_is_empty() {
        let store = ConfigStore::new();
        assert_eq!(store.list().count(), 0);
    }

    #[test]
    fn test_add_and_get_config() {
        let mut store = ConfigStore::new();
        store.add("test", sample_config());

        let config = store.get("test");
        assert!(config.is_some());

        let config = config.unwrap();
        assert_eq!(config.user.email, "test@example.com");
        assert_eq!(config.user.name, "Test User");
        assert_eq!(config.user.signoff, Some("test".into()));
        assert!(config.commit.gpgsign);
    }

    #[test]
    fn test_get_nonexistent_config() {
        let store = ConfigStore::new();
        assert!(store.get("nonexistent").is_none());
    }

    #[test]
    fn test_remove_config() {
        let mut store = ConfigStore::new();
        store.add("test", sample_config());

        let removed = store.remove("test");
        assert!(removed.is_some());
        assert!(store.get("test").is_none());
    }

    #[test]
    fn test_remove_nonexistent_config() {
        let mut store = ConfigStore::new();
        let removed = store.remove("nonexistent");
        assert!(removed.is_none());
    }

    #[test]
    fn test_list_configs() {
        let mut store = ConfigStore::new();
        store.add("work", sample_config());
        store.add("personal", sample_config());

        let names: Vec<_> = store.list().collect();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&&"work".to_string()));
        assert!(names.contains(&&"personal".to_string()));
    }

    #[test]
    fn test_iter_configs() {
        let mut store = ConfigStore::new();
        store.add("work", sample_config());
        store.add("personal", sample_config());

        let configs: Vec<_> = store.iter().collect();
        assert_eq!(configs.len(), 2);
    }

    #[test]
    fn test_serialize_deserialize() {
        let mut store = ConfigStore::new();
        store.add("test", sample_config());

        let json = serde_json::to_string(&store).unwrap();
        let restored: ConfigStore = serde_json::from_str(&json).unwrap();

        let config = restored.get("test").unwrap();
        assert_eq!(config.user.email, "test@example.com");
    }

    #[test]
    fn test_save_and_load() {
        let temp_dir = std::env::temp_dir().join("gam_test");
        let temp_file = temp_dir.join("config.json");

        // Clean up before test
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // Create and save store
        let mut store = ConfigStore::new();
        store.add("test", sample_config());

        let json = serde_json::to_string_pretty(&store).unwrap();
        fs::write(&temp_file, &json).unwrap();

        // Load and verify
        let contents = fs::read_to_string(&temp_file).unwrap();
        let loaded: ConfigStore = serde_json::from_str(&contents).unwrap();

        let config = loaded.get("test").unwrap();
        assert_eq!(config.user.email, "test@example.com");
        assert_eq!(config.user.name, "Test User");

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_config_path_exists() {
        // Just verify that config_path returns Some on supported platforms
        let path = ConfigStore::config_path();
        assert!(path.is_some());
        assert!(path.unwrap().ends_with("config.json"));
    }

    #[test]
    fn test_config_dir_exists() {
        let dir = ConfigStore::config_dir();
        assert!(dir.is_some());
        assert!(dir.unwrap().ends_with("gamm"));
    }

    #[test]
    fn test_user_config_default() {
        let user = UserConfig::default();
        assert_eq!(user.email, "");
        assert_eq!(user.name, "");
        assert!(user.signoff.is_none());
    }

    #[test]
    fn test_commit_config_default() {
        let commit = CommitConfig::default();
        assert!(!commit.gpgsign);
    }

    #[test]
    fn test_git_config_default() {
        let config = GitConfig::default();
        assert_eq!(config.user.email, "");
        assert!(config.urls.is_empty());
        assert!(!config.commit.gpgsign);
    }
}
