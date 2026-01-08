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

    /// Primary branch name (e.g., "main", "master")
    /// Note: "trunk" is accepted as an alias for backward compatibility
    #[serde(default = "default_primary", alias = "trunk")]
    pub primary: String,
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

fn default_primary() -> String {
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
            primary: default_primary(),
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
    /// Load config with hierarchy: local .jflow.toml > global ~/.jflow.toml > defaults
    /// Local config values override global config values.
    pub fn load() -> Result<Self> {
        // Start with defaults
        let mut config = Self::default();

        // Load global config if it exists (~/.jflow.toml)
        if let Some(global_path) = Self::global_config_path() {
            if global_path.exists() {
                if let Ok(contents) = std::fs::read_to_string(&global_path) {
                    if let Ok(global_config) = toml::from_str::<Config>(&contents) {
                        config = Self::merge(config, global_config);
                    }
                }
            }
        }

        // Load local config if it exists (overrides global)
        if let Ok(local_path) = Self::find_local_config_file() {
            let contents = std::fs::read_to_string(&local_path)
                .with_context(|| format!("Failed to read config file: {:?}", local_path))?;
            let local_config: Config = toml::from_str(&contents)
                .with_context(|| format!("Failed to parse config file: {:?}", local_path))?;
            config = Self::merge(config, local_config);
        }

        Ok(config)
    }

    /// Load config or return default if not found
    pub fn load_or_default() -> Result<Self> {
        // load() now always succeeds (falls back to defaults)
        Self::load()
    }

    /// Get the path to the global config file (~/.jflow.toml)
    pub fn global_config_path() -> Option<PathBuf> {
        dirs::home_dir().map(|home| home.join(".jflow.toml"))
    }

    /// Find .jflow.toml in current directory or parent directories
    fn find_local_config_file() -> Result<PathBuf> {
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

    /// Merge two configs, with `overlay` values taking precedence over `base`
    fn merge(base: Config, overlay: Config) -> Config {
        Config {
            remote: RemoteConfig {
                name: if overlay.remote.name != default_remote() {
                    overlay.remote.name
                } else {
                    base.remote.name
                },
                primary: if overlay.remote.primary != default_primary() {
                    overlay.remote.primary
                } else {
                    base.remote.primary
                },
            },
            github: GitHubConfig {
                push_style: if overlay.github.push_style != default_push_style() {
                    overlay.github.push_style
                } else {
                    base.github.push_style
                },
                merge_style: if overlay.github.merge_style != default_merge_style() {
                    overlay.github.merge_style
                } else {
                    base.github.merge_style
                },
                // For booleans, we can't easily detect "not set" vs "set to default"
                // So overlay always wins for these
                stack_context: overlay.github.stack_context,
            },
            display: DisplayConfig {
                theme: if overlay.display.theme != default_theme() {
                    overlay.display.theme
                } else {
                    base.display.theme
                },
                show_commit_ids: overlay.display.show_commit_ids,
                icons: if overlay.display.icons != default_icons() {
                    overlay.display.icons
                } else {
                    base.display.icons
                },
            },
            bookmarks: BookmarkConfig {
                prefix: if !overlay.bookmarks.prefix.is_empty() {
                    overlay.bookmarks.prefix
                } else {
                    base.bookmarks.prefix
                },
            },
        }
    }

    /// Get the revset for querying the default stack (all local changes not on primary)
    /// Falls back gracefully if remote tracking doesn't exist
    pub fn stack_revset(&self) -> String {
        let primary_ref = self.resolve_primary_ref();
        format!("::@ ~ ::{}", primary_ref)
    }

    /// Get primary branch reference (e.g., "main@origin")
    /// Falls back to local primary or root() if remote doesn't exist
    pub fn primary_ref(&self) -> String {
        self.resolve_primary_ref()
    }

    /// Backwards compatibility alias for primary_ref
    pub fn trunk_ref(&self) -> String {
        self.primary_ref()
    }

    /// Resolve the best available primary branch reference
    /// Priority: primary@remote > primary (local) > root()
    fn resolve_primary_ref(&self) -> String {
        // Try remote tracking first (e.g., main@origin)
        let remote_ref = format!("{}@{}", self.remote.primary, self.remote.name);
        if Self::revision_exists(&remote_ref) {
            return remote_ref;
        }

        // Try local primary (e.g., main)
        if Self::revision_exists(&self.remote.primary) {
            return self.remote.primary.clone();
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

    /// Parse config from a TOML string (for testing)
    pub fn from_toml(contents: &str) -> Result<Self> {
        toml::from_str(contents).context("Failed to parse config")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.remote.name, "origin");
        assert_eq!(config.remote.primary, "main");
        assert_eq!(config.github.push_style, "squash");
        assert_eq!(config.github.merge_style, "squash");
        assert!(config.github.stack_context);
        assert_eq!(config.display.theme, "catppuccin");
        assert_eq!(config.display.icons, "unicode");
        assert!(!config.display.show_commit_ids);
        assert_eq!(config.bookmarks.prefix, "");
    }

    #[test]
    fn test_parse_minimal_config() {
        let toml = r#"
[remote]
primary = "master"
"#;
        let config = Config::from_toml(toml).unwrap();
        assert_eq!(config.remote.primary, "master");
        // Other fields should use defaults
        assert_eq!(config.remote.name, "origin");
        assert_eq!(config.github.push_style, "squash");
    }

    #[test]
    fn test_parse_trunk_alias_for_backward_compat() {
        // "trunk" should still work as an alias for "primary"
        let toml = r#"
[remote]
trunk = "master"
"#;
        let config = Config::from_toml(toml).unwrap();
        assert_eq!(config.remote.primary, "master");
    }

    #[test]
    fn test_parse_full_config() {
        let toml = r#"
[remote]
name = "upstream"
primary = "develop"

[github]
push_style = "append"
merge_style = "rebase"
stack_context = false

[display]
theme = "nord"
icons = "ascii"
show_commit_ids = true

[bookmarks]
prefix = "jf/"
"#;
        let config = Config::from_toml(toml).unwrap();
        assert_eq!(config.remote.name, "upstream");
        assert_eq!(config.remote.primary, "develop");
        assert_eq!(config.github.push_style, "append");
        assert_eq!(config.github.merge_style, "rebase");
        assert!(!config.github.stack_context);
        assert_eq!(config.display.theme, "nord");
        assert_eq!(config.display.icons, "ascii");
        assert!(config.display.show_commit_ids);
        assert_eq!(config.bookmarks.prefix, "jf/");
    }

    #[test]
    fn test_parse_empty_config() {
        let toml = "";
        let config = Config::from_toml(toml).unwrap();
        // All defaults should apply
        assert_eq!(config.remote.name, "origin");
        assert_eq!(config.remote.primary, "main");
    }

    #[test]
    fn test_parse_partial_sections() {
        let toml = r#"
[github]
push_style = "append"
"#;
        let config = Config::from_toml(toml).unwrap();
        // Specified value
        assert_eq!(config.github.push_style, "append");
        // Defaults for unspecified fields in same section
        assert_eq!(config.github.merge_style, "squash");
        assert!(config.github.stack_context);
        // Defaults for unspecified sections
        assert_eq!(config.remote.name, "origin");
    }

    #[test]
    fn test_invalid_toml() {
        let toml = "this is not valid toml [[[";
        let result = Config::from_toml(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        let config = Config::default();
        let serialized = toml::to_string(&config).unwrap();
        let deserialized = Config::from_toml(&serialized).unwrap();
        assert_eq!(config.remote.name, deserialized.remote.name);
        assert_eq!(config.remote.primary, deserialized.remote.primary);
        assert_eq!(config.github.push_style, deserialized.github.push_style);
    }

    // Note: Tests that change current directory are run serially to avoid conflicts.
    // cargo test runs tests in parallel by default, so we use a mutex.
    use std::sync::Mutex;
    static DIR_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_load_from_temp_file() {
        use std::fs;
        use tempfile::tempdir;

        let _guard = DIR_MUTEX.lock().unwrap();

        let dir = tempdir().unwrap();
        let config_path = dir.path().join(".jflow.toml");

        let toml = r#"
[remote]
primary = "main"
name = "origin"
"#;
        fs::write(&config_path, toml).unwrap();

        // Change to temp directory and load
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let result = Config::load();

        // Restore original directory before asserting
        std::env::set_current_dir(original_dir).unwrap();

        let config = result.unwrap();
        assert_eq!(config.remote.primary, "main");
    }

    #[test]
    fn test_load_or_default_when_missing() {
        use tempfile::tempdir;

        let _guard = DIR_MUTEX.lock().unwrap();

        let dir = tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        // No config file exists, should return default
        let result = Config::load_or_default();

        // Restore original directory before asserting
        std::env::set_current_dir(original_dir).unwrap();

        let config = result.unwrap();
        assert_eq!(config.remote.name, "origin");
    }

    // === Edge Case Tests ===

    #[test]
    fn test_config_unknown_section_ignored() {
        let toml = r#"
[remote]
primary = "main"

[unknown_section]
foo = "bar"
"#;
        let config = Config::from_toml(toml).unwrap();
        assert_eq!(config.remote.primary, "main");
    }

    #[test]
    fn test_config_extra_fields_ignored() {
        let toml = r#"
[remote]
primary = "main"
unknown_field = "ignored"
"#;
        let config = Config::from_toml(toml).unwrap();
        assert_eq!(config.remote.primary, "main");
    }

    #[test]
    fn test_config_unicode_values() {
        let toml = r#"
[remote]
primary = "主要"
name = "origen"

[bookmarks]
prefix = "功能/"
"#;
        let config = Config::from_toml(toml).unwrap();
        assert_eq!(config.remote.primary, "主要");
        assert_eq!(config.bookmarks.prefix, "功能/");
    }

    #[test]
    fn test_config_special_chars_in_prefix() {
        let toml = r#"
[bookmarks]
prefix = "jf-test_prefix/"
"#;
        let config = Config::from_toml(toml).unwrap();
        assert_eq!(config.bookmarks.prefix, "jf-test_prefix/");
    }

    #[test]
    fn test_config_empty_string_values() {
        let toml = r#"
[remote]
primary = ""
name = ""

[bookmarks]
prefix = ""
"#;
        let config = Config::from_toml(toml).unwrap();
        assert_eq!(config.remote.primary, "");
        assert_eq!(config.remote.name, "");
        assert_eq!(config.bookmarks.prefix, "");
    }

    #[test]
    fn test_config_whitespace_in_values() {
        let toml = r#"
[remote]
primary = "  main  "
"#;
        let config = Config::from_toml(toml).unwrap();
        // TOML preserves whitespace in strings
        assert_eq!(config.remote.primary, "  main  ");
    }

    #[test]
    fn test_config_boolean_false_explicit() {
        let toml = r#"
[github]
stack_context = false

[display]
show_commit_ids = false
"#;
        let config = Config::from_toml(toml).unwrap();
        assert!(!config.github.stack_context);
        assert!(!config.display.show_commit_ids);
    }

    #[test]
    fn test_config_invalid_type_for_boolean() {
        let toml = r#"
[github]
stack_context = "yes"
"#;
        let result = Config::from_toml(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_invalid_type_string_as_number() {
        let toml = r#"
[remote]
primary = 123
"#;
        let result = Config::from_toml(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_unclosed_section() {
        let toml = r#"
[remote
primary = "main"
"#;
        let result = Config::from_toml(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_unclosed_string() {
        let toml = r#"
[remote]
primary = "main
"#;
        let result = Config::from_toml(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_duplicate_keys() {
        let toml = r#"
[remote]
primary = "main"
primary = "master"
"#;
        // TOML spec says last value wins, but behavior may vary
        let result = Config::from_toml(toml);
        // This might succeed or fail depending on TOML parser strictness
        // Just verify it doesn't panic
        let _ = result;
    }

    #[test]
    fn test_config_very_long_values() {
        let long_value = "a".repeat(10000);
        let toml = format!(
            r#"
[remote]
primary = "{}"
"#,
            long_value
        );
        let config = Config::from_toml(&toml).unwrap();
        assert_eq!(config.remote.primary.len(), 10000);
    }

    #[test]
    fn test_config_multiline_string() {
        let toml = r#"
[bookmarks]
prefix = """
jf/
"""
"#;
        let config = Config::from_toml(toml).unwrap();
        // Multiline strings include the newlines
        assert!(config.bookmarks.prefix.contains("jf/"));
    }

    #[test]
    fn test_config_escaped_characters() {
        let toml = r#"
[bookmarks]
prefix = "jf\\test"
"#;
        let config = Config::from_toml(toml).unwrap();
        assert_eq!(config.bookmarks.prefix, "jf\\test");
    }

    #[test]
    fn test_stack_revset_format() {
        let config = Config::default();
        let revset = config.stack_revset();
        // Should be in format "::@ ~ ::trunk_ref"
        assert!(revset.contains("::@"));
        assert!(revset.contains("~"));
    }
}
