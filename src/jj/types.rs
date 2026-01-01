use serde::{Deserialize, Serialize};

/// A change in the jj repository
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Change {
    pub change_id: String,
    pub commit_id: String,

    #[serde(default)]
    pub description: String,

    #[serde(default)]
    pub author: Author,

    #[serde(default)]
    pub bookmarks: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Author {
    #[serde(default)]
    pub name: String,

    #[serde(default)]
    pub email: String,
}

/// Sync state between local bookmark and remote
#[derive(Debug, Clone, Default)]
pub enum BookmarkSyncState {
    /// No bookmark on this change
    #[default]
    NoBookmark,
    /// Local-only bookmark (not pushed to remote)
    LocalOnly,
    /// Synced with remote (same commit)
    Synced,
    /// Local is ahead of remote
    Ahead {
        count: usize,
    },
    /// Local is behind remote
    Behind {
        count: usize,
    },
    /// Local and remote have diverged
    Diverged {
        local_ahead: usize,
        remote_ahead: usize,
        fork_point: Option<String>, // change_id of common ancestor
    },
}

/// A change with additional status information
#[derive(Debug, Clone)]
pub struct ChangeWithStatus {
    pub change: Change,
    pub bookmark: Option<String>,
    pub is_working: bool,
    /// True if this change has a bookmark that's tracked on remote
    pub has_remote: bool,
    /// Sync state between local and remote
    pub sync_state: BookmarkSyncState,
}
