use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub remote: RemoteConfig,

    #[serde(default)]
    pub github: GitHubConfig,

    #[serde(default)]
    pub display: DisplayConfig,

    #[serde(default)]
    pub bookmarks: BookmarkConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RemoteConfig {
    /// Remote name (e.g., "origin")
    #[serde(default = "default_remote")]
    pub name: String,

    /// Trunk branch name (e.g., "main", "master")
    #[serde(default = "default_trunk")]
    pub trunk: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GitHubConfig {
    /// Push style: "squash" (force-push) or "append" (incremental commits)
    #[serde(default = "default_push_style")]
    pub push_style: String,

    /// Merge style: "squash", "merge", or "rebase"
    #[serde(default = "default_merge_style")]
    pub merge_style: String,

    /// Add stack context to PR descriptions
    #[serde(default = "default_true")]
    pub stack_context: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DisplayConfig {
    /// Theme: catppuccin, nord, dracula, default
    #[serde(default = "default_theme")]
    pub theme: String,

    /// Show git commit hashes
    #[serde(default)]
    pub show_commit_ids: bool,

    /// Icons: unicode, ascii
    #[serde(default = "default_icons")]
    pub icons: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BookmarkConfig {
    /// Prefix for bookmarks (e.g., "" or "jf/")
    #[serde(default)]
    pub prefix: String,
}

// Default values
fn default_remote() -> String {
    "origin".to_string()
}

fn default_trunk() -> String {
    "main".to_string()
}

fn default_push_style() -> String {
    "squash".to_string()
}

fn default_merge_style() -> String {
    "squash".to_string()
}

fn default_theme() -> String {
    "catppuccin".to_string()
}

fn default_icons() -> String {
    "unicode".to_string()
}

fn default_true() -> bool {
    true
}

impl Default for RemoteConfig {
    fn default() -> Self {
        Self {
            name: default_remote(),
            trunk: default_trunk(),
        }
    }
}

impl Default for GitHubConfig {
    fn default() -> Self {
        Self {
            push_style: default_push_style(),
            merge_style: default_merge_style(),
            stack_context: true,
        }
    }
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            show_commit_ids: false,
            icons: default_icons(),
        }
    }
}

impl Default for BookmarkConfig {
    fn default() -> Self {
        Self {
            prefix: String::new(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            remote: RemoteConfig::default(),
            github: GitHubConfig::default(),
            display: DisplayConfig::default(),
            bookmarks: BookmarkConfig::default(),
        }
    }
}

impl Config {
    /// Load config from .jflow.toml in current directory or parent directories
    pub fn load() -> Result<Self> {
        let config_path = Self::find_config_file()?;
        let contents = std::fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config file: {:?}", config_path))?;

        toml::from_str(&contents)
            .with_context(|| format!("Failed to parse config file: {:?}", config_path))
    }

    /// Load config or return default if not found
    pub fn load_or_default() -> Result<Self> {
        match Self::load() {
            Ok(config) => Ok(config),
            Err(_) => Ok(Self::default()),
        }
    }

    /// Find .jflow.toml in current directory or parent directories
    fn find_config_file() -> Result<PathBuf> {
        let mut current_dir = std::env::current_dir()?;

        loop {
            let config_path = current_dir.join(".jflow.toml");
            if config_path.exists() {
                return Ok(config_path);
            }

            if !current_dir.pop() {
                anyhow::bail!("No .jflow.toml found in current directory or parent directories");
            }
        }
    }

    /// Get the revset for querying the default stack (all local changes not on trunk)
    /// Falls back gracefully if remote tracking doesn't exist
    pub fn stack_revset(&self) -> String {
        let trunk_ref = self.resolve_trunk_ref();
        format!("::@ ~ ::{}", trunk_ref)
    }

    /// Get trunk reference (e.g., "main@origin")
    /// Falls back to local trunk or root() if remote doesn't exist
    pub fn trunk_ref(&self) -> String {
        self.resolve_trunk_ref()
    }

    /// Resolve the best available trunk reference
    /// Priority: trunk@remote > trunk (local) > root()
    fn resolve_trunk_ref(&self) -> String {
        // Try remote tracking first (e.g., main@origin)
        let remote_ref = format!("{}@{}", self.remote.trunk, self.remote.name);
        if Self::revision_exists(&remote_ref) {
            return remote_ref;
        }

        // Try local trunk (e.g., main)
        if Self::revision_exists(&self.remote.trunk) {
            return self.remote.trunk.clone();
        }

        // Fall back to root
        "root()".to_string()
    }

    /// Check if a revision exists in the jj repo
    fn revision_exists(rev: &str) -> bool {
        use std::process::Command;

        Command::new("jj")
            .args(["log", "-r", rev, "--limit", "1", "--no-graph", "-T", "''"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}
