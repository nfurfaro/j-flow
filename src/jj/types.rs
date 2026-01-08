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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_change_deserialize_full() {
        let json = r#"{
            "change_id": "abc123",
            "commit_id": "def456",
            "description": "Add feature",
            "author": {"name": "Test User", "email": "test@example.com"},
            "bookmarks": ["feature-branch", "backup"]
        }"#;

        let change: Change = serde_json::from_str(json).unwrap();
        assert_eq!(change.change_id, "abc123");
        assert_eq!(change.commit_id, "def456");
        assert_eq!(change.description, "Add feature");
        assert_eq!(change.author.name, "Test User");
        assert_eq!(change.author.email, "test@example.com");
        assert_eq!(change.bookmarks, vec!["feature-branch", "backup"]);
    }

    #[test]
    fn test_change_deserialize_minimal() {
        let json = r#"{
            "change_id": "abc123",
            "commit_id": "def456"
        }"#;

        let change: Change = serde_json::from_str(json).unwrap();
        assert_eq!(change.change_id, "abc123");
        assert_eq!(change.commit_id, "def456");
        assert_eq!(change.description, "");
        assert_eq!(change.author.name, "");
        assert_eq!(change.author.email, "");
        assert!(change.bookmarks.is_empty());
    }

    #[test]
    fn test_change_deserialize_empty_bookmarks() {
        let json = r#"{
            "change_id": "abc123",
            "commit_id": "def456",
            "description": "Test",
            "author": {"name": "", "email": ""},
            "bookmarks": []
        }"#;

        let change: Change = serde_json::from_str(json).unwrap();
        assert!(change.bookmarks.is_empty());
    }

    #[test]
    fn test_change_serialize_roundtrip() {
        let change = Change {
            change_id: "abc123".to_string(),
            commit_id: "def456".to_string(),
            description: "Test change".to_string(),
            author: Author {
                name: "Test".to_string(),
                email: "test@test.com".to_string(),
            },
            bookmarks: vec!["branch1".to_string()],
        };

        let json = serde_json::to_string(&change).unwrap();
        let parsed: Change = serde_json::from_str(&json).unwrap();

        assert_eq!(change.change_id, parsed.change_id);
        assert_eq!(change.commit_id, parsed.commit_id);
        assert_eq!(change.description, parsed.description);
        assert_eq!(change.bookmarks, parsed.bookmarks);
    }

    #[test]
    fn test_bookmark_sync_state_default() {
        let state = BookmarkSyncState::default();
        assert!(matches!(state, BookmarkSyncState::NoBookmark));
    }

    #[test]
    fn test_bookmark_sync_state_variants() {
        // Test each variant can be constructed
        let _ = BookmarkSyncState::NoBookmark;
        let _ = BookmarkSyncState::LocalOnly;
        let _ = BookmarkSyncState::Synced;
        let _ = BookmarkSyncState::Ahead { count: 5 };
        let _ = BookmarkSyncState::Behind { count: 3 };
        let _ = BookmarkSyncState::Diverged {
            local_ahead: 2,
            remote_ahead: 3,
            fork_point: Some("xyz".to_string()),
        };
    }

    #[test]
    fn test_change_with_status_construction() {
        let change = Change {
            change_id: "abc".to_string(),
            commit_id: "def".to_string(),
            description: "Test".to_string(),
            author: Author::default(),
            bookmarks: vec![],
        };

        let status = ChangeWithStatus {
            change,
            bookmark: Some("feature".to_string()),
            is_working: true,
            has_remote: true,
            sync_state: BookmarkSyncState::Ahead { count: 2 },
        };

        assert_eq!(status.bookmark, Some("feature".to_string()));
        assert!(status.is_working);
        assert!(status.has_remote);
        assert!(matches!(status.sync_state, BookmarkSyncState::Ahead { count: 2 }));
    }

    #[test]
    fn test_author_default() {
        let author = Author::default();
        assert_eq!(author.name, "");
        assert_eq!(author.email, "");
    }

    #[test]
    fn test_parse_jj_output_format() {
        // This tests the exact JSON format jj produces
        let jj_output = r#"{"change_id":"uyxvnszr","commit_id":"12da6551","description":"","author":{"name":"Nick Furfaro","email":"nfurfaro33@gmail.com"},"bookmarks":[]}"#;

        let change: Change = serde_json::from_str(jj_output).unwrap();
        assert_eq!(change.change_id, "uyxvnszr");
        assert_eq!(change.description, "");
    }

    // === Edge Case Tests ===

    #[test]
    fn test_change_empty_ids() {
        let json = r#"{"change_id":"","commit_id":"","description":"","author":{"name":"","email":""},"bookmarks":[]}"#;
        let change: Change = serde_json::from_str(json).unwrap();
        assert_eq!(change.change_id, "");
        assert_eq!(change.commit_id, "");
    }

    #[test]
    fn test_change_very_long_ids() {
        let long_id = "a".repeat(1000);
        let json = format!(
            r#"{{"change_id":"{}","commit_id":"{}","description":"","author":{{"name":"","email":""}},"bookmarks":[]}}"#,
            long_id, long_id
        );
        let change: Change = serde_json::from_str(&json).unwrap();
        assert_eq!(change.change_id.len(), 1000);
    }

    #[test]
    fn test_change_unicode_description() {
        let json = r#"{"change_id":"abc","commit_id":"def","description":"添加功能 🎉","author":{"name":"李明","email":"test@test.com"},"bookmarks":[]}"#;
        let change: Change = serde_json::from_str(json).unwrap();
        assert_eq!(change.description, "添加功能 🎉");
        assert_eq!(change.author.name, "李明");
    }

    #[test]
    fn test_change_escaped_quotes_in_description() {
        let json = r#"{"change_id":"abc","commit_id":"def","description":"Fix \"bug\" in code","author":{"name":"","email":""},"bookmarks":[]}"#;
        let change: Change = serde_json::from_str(json).unwrap();
        assert_eq!(change.description, r#"Fix "bug" in code"#);
    }

    #[test]
    fn test_change_backslash_in_description() {
        let json = r#"{"change_id":"abc","commit_id":"def","description":"Path: C:\\Users\\test","author":{"name":"","email":""},"bookmarks":[]}"#;
        let change: Change = serde_json::from_str(json).unwrap();
        assert_eq!(change.description, r"Path: C:\Users\test");
    }

    #[test]
    fn test_change_newline_in_description() {
        let json = r#"{"change_id":"abc","commit_id":"def","description":"Line1\nLine2","author":{"name":"","email":""},"bookmarks":[]}"#;
        let change: Change = serde_json::from_str(json).unwrap();
        assert!(change.description.contains('\n'));
    }

    #[test]
    fn test_change_tab_in_description() {
        let json = r#"{"change_id":"abc","commit_id":"def","description":"Col1\tCol2","author":{"name":"","email":""},"bookmarks":[]}"#;
        let change: Change = serde_json::from_str(json).unwrap();
        assert!(change.description.contains('\t'));
    }

    #[test]
    fn test_change_very_long_description() {
        let long_desc = "x".repeat(100000);
        let json = format!(
            r#"{{"change_id":"abc","commit_id":"def","description":"{}","author":{{"name":"","email":""}},"bookmarks":[]}}"#,
            long_desc
        );
        let change: Change = serde_json::from_str(&json).unwrap();
        assert_eq!(change.description.len(), 100000);
    }

    #[test]
    fn test_change_many_bookmarks() {
        let bookmarks: Vec<String> = (0..100).map(|i| format!("bookmark-{}", i)).collect();
        let bookmarks_json = serde_json::to_string(&bookmarks).unwrap();
        let json = format!(
            r#"{{"change_id":"abc","commit_id":"def","description":"","author":{{"name":"","email":""}},"bookmarks":{}}}"#,
            bookmarks_json
        );
        let change: Change = serde_json::from_str(&json).unwrap();
        assert_eq!(change.bookmarks.len(), 100);
    }

    #[test]
    fn test_change_bookmarks_with_special_chars() {
        let json = r#"{"change_id":"abc","commit_id":"def","description":"","author":{"name":"","email":""},"bookmarks":["feature/test","fix-bug","user@branch"]}"#;
        let change: Change = serde_json::from_str(json).unwrap();
        assert_eq!(change.bookmarks.len(), 3);
        assert!(change.bookmarks.contains(&"feature/test".to_string()));
        assert!(change.bookmarks.contains(&"user@branch".to_string()));
    }

    #[test]
    fn test_change_author_special_chars() {
        let json = r#"{"change_id":"abc","commit_id":"def","description":"","author":{"name":"O'Brien, José","email":"test+tag@example.com"},"bookmarks":[]}"#;
        let change: Change = serde_json::from_str(json).unwrap();
        assert_eq!(change.author.name, "O'Brien, José");
        assert_eq!(change.author.email, "test+tag@example.com");
    }

    #[test]
    fn test_sync_state_ahead_with_zero() {
        // Edge case: Ahead with 0 count (shouldn't normally happen)
        let state = BookmarkSyncState::Ahead { count: 0 };
        assert!(matches!(state, BookmarkSyncState::Ahead { count: 0 }));
    }

    #[test]
    fn test_sync_state_behind_with_zero() {
        let state = BookmarkSyncState::Behind { count: 0 };
        assert!(matches!(state, BookmarkSyncState::Behind { count: 0 }));
    }

    #[test]
    fn test_sync_state_diverged_with_zeros() {
        let state = BookmarkSyncState::Diverged {
            local_ahead: 0,
            remote_ahead: 0,
            fork_point: None,
        };
        assert!(matches!(
            state,
            BookmarkSyncState::Diverged {
                local_ahead: 0,
                remote_ahead: 0,
                ..
            }
        ));
    }

    #[test]
    fn test_sync_state_diverged_large_counts() {
        let state = BookmarkSyncState::Diverged {
            local_ahead: usize::MAX,
            remote_ahead: usize::MAX,
            fork_point: Some("abc".to_string()),
        };
        if let BookmarkSyncState::Diverged { local_ahead, remote_ahead, .. } = state {
            assert_eq!(local_ahead, usize::MAX);
            assert_eq!(remote_ahead, usize::MAX);
        }
    }

    #[test]
    fn test_change_with_status_no_bookmark() {
        let change = Change {
            change_id: "abc".to_string(),
            commit_id: "def".to_string(),
            description: "Test".to_string(),
            author: Author::default(),
            bookmarks: vec![],
        };
        let status = ChangeWithStatus {
            change,
            bookmark: None,
            is_working: false,
            has_remote: false,
            sync_state: BookmarkSyncState::NoBookmark,
        };
        assert!(status.bookmark.is_none());
        assert!(matches!(status.sync_state, BookmarkSyncState::NoBookmark));
    }
}
