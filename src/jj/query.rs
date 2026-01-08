use anyhow::{Context, Result};
use std::process::Command;

use super::types::{BookmarkSyncState, Change, ChangeWithStatus};

/// A bookmark from jj with sync information
struct Bookmark {
    name: String,
    change_id: String,
    has_remote: bool,
    /// Sync state with remote
    sync_state: BookmarkSyncState,
}

/// Execute jj command and return output
pub fn run_jj(args: &[&str]) -> Result<String> {
    let output = Command::new("jj")
        .args(args)
        .output()
        .context("Failed to execute jj command. Is jj installed?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("jj command failed: {}", stderr);
    }

    Ok(String::from_utf8(output.stdout)?)
}

/// Query changes using a revset
pub fn query_changes(revset: &str) -> Result<Vec<Change>> {
    // jj template syntax uses concat() and string literals
    let template = r#"concat(
        "{\"change_id\":\"", change_id, "\",",
        "\"commit_id\":\"", commit_id, "\",",
        "\"description\":\"", description.first_line(), "\",",
        "\"author\":{\"name\":\"", author.name(), "\",\"email\":\"", author.email(), "\"},",
        "\"bookmarks\":[", bookmarks.map(|b| concat("\"", b.name(), "\"")).join(","), "]",
        "}\n"
    )"#;

    let output = run_jj(&["log", "-r", revset, "-T", template, "--no-graph"])?;

    // Parse each line as JSON
    let mut changes = Vec::new();
    for line in output.lines() {
        if line.trim().is_empty() {
            continue;
        }

        match serde_json::from_str::<Change>(line) {
            Ok(change) => changes.push(change),
            Err(e) => {
                eprintln!("Warning: Failed to parse change: {}", e);
                eprintln!("Line: {}", line);
            }
        }
    }

    Ok(changes)
}

/// Raw bookmark entry from jj
#[derive(Debug, serde::Deserialize)]
pub struct BookmarkEntry {
    name: String,
    remote: Option<String>,
    change_id: Option<String>,
    synced: bool,
    ahead: Option<usize>,
    behind: Option<usize>,
}

/// Get all bookmarks with sync state
fn query_bookmarks(remote_name: &str) -> Result<Vec<Bookmark>> {
    // Use jj template to get structured bookmark data
    // Use self.tracking_present() to check if this is a tracked remote ref before accessing tracking counts
    let template = r#"concat(
        "{\"name\":\"", name, "\",",
        "\"remote\":", if(remote, concat("\"", remote, "\""), "null"), ",",
        "\"change_id\":", if(normal_target, concat("\"", normal_target.change_id().short(), "\""), "null"), ",",
        "\"synced\":", self.synced(), ",",
        "\"ahead\":", if(self.tracking_present(), tracking_ahead_count.exact(), "null"), ",",
        "\"behind\":", if(self.tracking_present(), tracking_behind_count.exact(), "null"),
        "}\n"
    )"#;

    let output = run_jj(&["bookmark", "list", "--all", "-T", template])?;

    // Parse JSON entries
    let mut entries: Vec<BookmarkEntry> = Vec::new();
    for line in output.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<BookmarkEntry>(line) {
            Ok(entry) => entries.push(entry),
            Err(e) => {
                eprintln!("Warning: Failed to parse bookmark entry: {}", e);
                eprintln!("Line: {}", line);
            }
        }
    }

    // Group entries by bookmark name
    // For each local bookmark, find the corresponding remote tracking entry
    let mut bookmarks = Vec::new();

    // Get local bookmarks (remote is null) that have a valid change_id
    // Deleted bookmarks have change_id=null, skip them
    let local_entries: Vec<_> = entries
        .iter()
        .filter(|e| e.remote.is_none() && e.change_id.is_some())
        .collect();

    for local in local_entries {
        // Find the corresponding remote entry (not @git)
        let remote_entry = entries.iter().find(|e| {
            e.name == local.name
                && e.remote.as_ref().map(|r| r == remote_name).unwrap_or(false)
        });

        let (has_remote, sync_state) = match remote_entry {
            Some(remote) => {
                let ahead = remote.behind.unwrap_or(0); // remote behind = local ahead
                let behind = remote.ahead.unwrap_or(0); // remote ahead = local behind

                let state = if remote.synced {
                    BookmarkSyncState::Synced
                } else if ahead > 0 && behind > 0 {
                    // Diverged - need to find fork point
                    let fork_point = find_fork_point(&local.name, remote_name);
                    BookmarkSyncState::Diverged {
                        local_ahead: ahead,
                        remote_ahead: behind,
                        fork_point,
                    }
                } else if ahead > 0 {
                    BookmarkSyncState::Ahead { count: ahead }
                } else if behind > 0 {
                    BookmarkSyncState::Behind { count: behind }
                } else {
                    BookmarkSyncState::Synced
                };

                (true, state)
            }
            None => (false, BookmarkSyncState::LocalOnly),
        };

        bookmarks.push(Bookmark {
            name: local.name.clone(),
            change_id: local.change_id.clone().unwrap_or_default(),
            has_remote,
            sync_state,
        });
    }

    Ok(bookmarks)
}

/// Find the fork point (common ancestor) between local and remote bookmark
fn find_fork_point(bookmark: &str, remote: &str) -> Option<String> {
    let remote_ref = format!("{}@{}", bookmark, remote);
    // Use revset to find common ancestor
    let revset = format!("heads(::({}) & ::({}))", bookmark, remote_ref);
    match run_jj(&["log", "-r", &revset, "-T", "change_id.short()", "--no-graph", "--limit", "1"]) {
        Ok(output) => {
            let id = output.trim().to_string();
            if id.is_empty() {
                None
            } else {
                Some(id)
            }
        }
        Err(_) => None,
    }
}

/// Get current working copy change ID
fn get_working_copy_id() -> Result<String> {
    let output = run_jj(&["log", "-r", "@", "-T", "change_id", "--no-graph"])?;
    Ok(output.trim().to_string())
}

/// Get stack with status information
pub fn get_stack(revset: &str, remote_name: &str) -> Result<Vec<ChangeWithStatus>> {
    let changes = query_changes(revset)?;
    let bookmarks = query_bookmarks(remote_name)?;
    let working_id = get_working_copy_id()?;

    // Match bookmarks to changes
    // Note: bookmark list shows short IDs, changes have full IDs
    // Match by prefix (but skip empty change_ids which would match everything)
    let mut result = Vec::new();
    for change in changes {
        let matched_bookmark = bookmarks
            .iter()
            .find(|b| !b.change_id.is_empty() && change.change_id.starts_with(&b.change_id));

        let bookmark = matched_bookmark.map(|b| b.name.clone());
        let has_remote = matched_bookmark.map(|b| b.has_remote).unwrap_or(false);
        let sync_state = matched_bookmark
            .map(|b| b.sync_state.clone())
            .unwrap_or(BookmarkSyncState::NoBookmark);
        let is_working = change.change_id.starts_with(&working_id) || working_id.starts_with(&change.change_id);

        result.push(ChangeWithStatus {
            change,
            bookmark,
            is_working,
            has_remote,
            sync_state,
        });
    }

    Ok(result)
}

/// Check if jj is available
pub fn check_jj_available() -> Result<()> {
    Command::new("jj")
        .arg("--version")
        .output()
        .context("jj command not found. Please install jujutsu: https://github.com/martinvonz/jj")?;

    Ok(())
}

/// Create a bookmark at a specific change
pub fn create_bookmark(name: &str, change_id: &str) -> Result<()> {
    run_jj(&["bookmark", "create", name, "-r", change_id])?;
    Ok(())
}

/// Parse changes from jj log JSON output (for testing)
pub fn parse_changes_output(output: &str) -> Vec<Change> {
    let mut changes = Vec::new();
    for line in output.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(change) = serde_json::from_str::<Change>(line) {
            changes.push(change);
        }
    }
    changes
}

/// Parse bookmark entries from jj bookmark list JSON output (for testing)
pub fn parse_bookmark_entries(output: &str) -> Vec<BookmarkEntry> {
    let mut entries = Vec::new();
    for line in output.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(entry) = serde_json::from_str::<BookmarkEntry>(line) {
            entries.push(entry);
        }
    }
    entries
}

/// Compute sync state from bookmark entries (for testing)
pub fn compute_sync_state(
    _local: &BookmarkEntry,
    remote: Option<&BookmarkEntry>,
) -> BookmarkSyncState {
    match remote {
        Some(remote) => {
            let ahead = remote.behind.unwrap_or(0);
            let behind = remote.ahead.unwrap_or(0);

            if remote.synced {
                BookmarkSyncState::Synced
            } else if ahead > 0 && behind > 0 {
                BookmarkSyncState::Diverged {
                    local_ahead: ahead,
                    remote_ahead: behind,
                    fork_point: None, // Can't compute without jj access
                }
            } else if ahead > 0 {
                BookmarkSyncState::Ahead { count: ahead }
            } else if behind > 0 {
                BookmarkSyncState::Behind { count: behind }
            } else {
                BookmarkSyncState::Synced
            }
        }
        None => BookmarkSyncState::LocalOnly,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_changes_output_single() {
        let output = r#"{"change_id":"abc123","commit_id":"def456","description":"Add feature","author":{"name":"Test","email":"test@test.com"},"bookmarks":["main"]}"#;

        let changes = parse_changes_output(output);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].change_id, "abc123");
        assert_eq!(changes[0].description, "Add feature");
        assert_eq!(changes[0].bookmarks, vec!["main"]);
    }

    #[test]
    fn test_parse_changes_output_multiple() {
        let output = r#"{"change_id":"abc123","commit_id":"def456","description":"First","author":{"name":"","email":""},"bookmarks":[]}
{"change_id":"xyz789","commit_id":"uvw012","description":"Second","author":{"name":"","email":""},"bookmarks":["feature"]}"#;

        let changes = parse_changes_output(output);
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0].change_id, "abc123");
        assert_eq!(changes[1].change_id, "xyz789");
    }

    #[test]
    fn test_parse_changes_output_empty() {
        let output = "";
        let changes = parse_changes_output(output);
        assert!(changes.is_empty());
    }

    #[test]
    fn test_parse_changes_output_with_blank_lines() {
        let output = r#"
{"change_id":"abc123","commit_id":"def456","description":"Test","author":{"name":"","email":""},"bookmarks":[]}

"#;
        let changes = parse_changes_output(output);
        assert_eq!(changes.len(), 1);
    }

    #[test]
    fn test_parse_changes_output_skips_invalid() {
        let output = r#"{"change_id":"abc123","commit_id":"def456","description":"Valid","author":{"name":"","email":""},"bookmarks":[]}
not valid json
{"change_id":"xyz789","commit_id":"uvw012","description":"Also valid","author":{"name":"","email":""},"bookmarks":[]}"#;

        let changes = parse_changes_output(output);
        assert_eq!(changes.len(), 2);
    }

    #[test]
    fn test_parse_bookmark_entries_local() {
        let output = r#"{"name":"feature","remote":null,"change_id":"abc123","synced":false,"ahead":null,"behind":null}"#;

        let entries = parse_bookmark_entries(output);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "feature");
        assert!(entries[0].remote.is_none());
        assert_eq!(entries[0].change_id, Some("abc123".to_string()));
    }

    #[test]
    fn test_parse_bookmark_entries_remote() {
        let output = r#"{"name":"feature","remote":"origin","change_id":"abc123","synced":true,"ahead":0,"behind":0}"#;

        let entries = parse_bookmark_entries(output);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].remote, Some("origin".to_string()));
        assert!(entries[0].synced);
    }

    #[test]
    fn test_parse_bookmark_entries_with_ahead_behind() {
        let output = r#"{"name":"feature","remote":"origin","change_id":"abc123","synced":false,"ahead":3,"behind":2}"#;

        let entries = parse_bookmark_entries(output);
        assert_eq!(entries[0].ahead, Some(3));
        assert_eq!(entries[0].behind, Some(2));
    }

    #[test]
    fn test_compute_sync_state_synced() {
        let local = BookmarkEntry {
            name: "feature".to_string(),
            remote: None,
            change_id: Some("abc".to_string()),
            synced: false,
            ahead: None,
            behind: None,
        };
        let remote = BookmarkEntry {
            name: "feature".to_string(),
            remote: Some("origin".to_string()),
            change_id: Some("abc".to_string()),
            synced: true,
            ahead: Some(0),
            behind: Some(0),
        };

        let state = compute_sync_state(&local, Some(&remote));
        assert!(matches!(state, BookmarkSyncState::Synced));
    }

    #[test]
    fn test_compute_sync_state_ahead() {
        let local = BookmarkEntry {
            name: "feature".to_string(),
            remote: None,
            change_id: Some("abc".to_string()),
            synced: false,
            ahead: None,
            behind: None,
        };
        let remote = BookmarkEntry {
            name: "feature".to_string(),
            remote: Some("origin".to_string()),
            change_id: Some("xyz".to_string()),
            synced: false,
            ahead: Some(0),
            behind: Some(3), // remote behind = local ahead
        };

        let state = compute_sync_state(&local, Some(&remote));
        assert!(matches!(state, BookmarkSyncState::Ahead { count: 3 }));
    }

    #[test]
    fn test_compute_sync_state_behind() {
        let local = BookmarkEntry {
            name: "feature".to_string(),
            remote: None,
            change_id: Some("abc".to_string()),
            synced: false,
            ahead: None,
            behind: None,
        };
        let remote = BookmarkEntry {
            name: "feature".to_string(),
            remote: Some("origin".to_string()),
            change_id: Some("xyz".to_string()),
            synced: false,
            ahead: Some(2), // remote ahead = local behind
            behind: Some(0),
        };

        let state = compute_sync_state(&local, Some(&remote));
        assert!(matches!(state, BookmarkSyncState::Behind { count: 2 }));
    }

    #[test]
    fn test_compute_sync_state_diverged() {
        let local = BookmarkEntry {
            name: "feature".to_string(),
            remote: None,
            change_id: Some("abc".to_string()),
            synced: false,
            ahead: None,
            behind: None,
        };
        let remote = BookmarkEntry {
            name: "feature".to_string(),
            remote: Some("origin".to_string()),
            change_id: Some("xyz".to_string()),
            synced: false,
            ahead: Some(2),
            behind: Some(3),
        };

        let state = compute_sync_state(&local, Some(&remote));
        assert!(matches!(
            state,
            BookmarkSyncState::Diverged {
                local_ahead: 3,
                remote_ahead: 2,
                ..
            }
        ));
    }

    #[test]
    fn test_compute_sync_state_local_only() {
        let local = BookmarkEntry {
            name: "feature".to_string(),
            remote: None,
            change_id: Some("abc".to_string()),
            synced: false,
            ahead: None,
            behind: None,
        };

        let state = compute_sync_state(&local, None);
        assert!(matches!(state, BookmarkSyncState::LocalOnly));
    }

    // === Edge Case Tests ===

    #[test]
    fn test_parse_changes_output_whitespace_only() {
        let output = "   \n\t\n   ";
        let changes = parse_changes_output(output);
        assert!(changes.is_empty());
    }

    #[test]
    fn test_parse_changes_output_many_blank_lines() {
        let output = r#"


{"change_id":"abc","commit_id":"def","description":"Test","author":{"name":"","email":""},"bookmarks":[]}



{"change_id":"xyz","commit_id":"uvw","description":"Test2","author":{"name":"","email":""},"bookmarks":[]}


"#;
        let changes = parse_changes_output(output);
        assert_eq!(changes.len(), 2);
    }

    #[test]
    fn test_parse_changes_output_unicode() {
        let output = r#"{"change_id":"abc","commit_id":"def","description":"添加功能 🎉","author":{"name":"张三","email":"zhangsan@test.com"},"bookmarks":["功能分支"]}"#;
        let changes = parse_changes_output(output);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].description, "添加功能 🎉");
        assert_eq!(changes[0].author.name, "张三");
        assert_eq!(changes[0].bookmarks, vec!["功能分支"]);
    }

    #[test]
    fn test_parse_changes_output_empty_description() {
        let output = r#"{"change_id":"abc","commit_id":"def","description":"","author":{"name":"","email":""},"bookmarks":[]}"#;
        let changes = parse_changes_output(output);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].description, "");
    }

    #[test]
    fn test_parse_changes_output_special_chars_in_description() {
        let output = r#"{"change_id":"abc","commit_id":"def","description":"Fix \"bug\" with \\path","author":{"name":"O'Brien","email":"test@test.com"},"bookmarks":[]}"#;
        let changes = parse_changes_output(output);
        assert_eq!(changes.len(), 1);
        assert!(changes[0].description.contains("\"bug\""));
        assert!(changes[0].description.contains("\\path"));
    }

    #[test]
    fn test_parse_changes_output_many_bookmarks() {
        let bookmarks: Vec<String> = (0..50).map(|i| format!("bookmark-{}", i)).collect();
        let bookmarks_json = serde_json::to_string(&bookmarks).unwrap();
        let output = format!(
            r#"{{"change_id":"abc","commit_id":"def","description":"Test","author":{{"name":"","email":""}},"bookmarks":{}}}"#,
            bookmarks_json
        );
        let changes = parse_changes_output(&output);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].bookmarks.len(), 50);
    }

    #[test]
    fn test_parse_changes_output_truncated_json() {
        // Incomplete JSON should be skipped
        let output = r#"{"change_id":"abc","commit_id":"def","description":"Valid","author":{"name":"","email":""},"bookmarks":[]}
{"change_id":"xyz","commit_id":"#;
        let changes = parse_changes_output(output);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].change_id, "abc");
    }

    #[test]
    fn test_parse_bookmark_entries_empty_change_id() {
        let output = r#"{"name":"feature","remote":null,"change_id":null,"synced":false,"ahead":null,"behind":null}"#;
        let entries = parse_bookmark_entries(output);
        assert_eq!(entries.len(), 1);
        assert!(entries[0].change_id.is_none());
    }

    #[test]
    fn test_parse_bookmark_entries_at_git_remote() {
        // jj also shows @git entries
        let output = r#"{"name":"main","remote":"git","change_id":"abc123","synced":true,"ahead":0,"behind":0}"#;
        let entries = parse_bookmark_entries(output);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].remote, Some("git".to_string()));
    }

    #[test]
    fn test_parse_bookmark_entries_multiple_remotes() {
        let output = r#"{"name":"main","remote":null,"change_id":"abc123","synced":false,"ahead":null,"behind":null}
{"name":"main","remote":"origin","change_id":"abc123","synced":true,"ahead":0,"behind":0}
{"name":"main","remote":"upstream","change_id":"xyz789","synced":false,"ahead":2,"behind":1}"#;
        let entries = parse_bookmark_entries(output);
        assert_eq!(entries.len(), 3);
        // One local, two remotes
        let local_count = entries.iter().filter(|e| e.remote.is_none()).count();
        let remote_count = entries.iter().filter(|e| e.remote.is_some()).count();
        assert_eq!(local_count, 1);
        assert_eq!(remote_count, 2);
    }

    #[test]
    fn test_parse_bookmark_entries_special_bookmark_names() {
        let output = r#"{"name":"feature/add-login","remote":null,"change_id":"abc","synced":false,"ahead":null,"behind":null}
{"name":"fix-bug#123","remote":null,"change_id":"def","synced":false,"ahead":null,"behind":null}
{"name":"user@work","remote":null,"change_id":"ghi","synced":false,"ahead":null,"behind":null}"#;
        let entries = parse_bookmark_entries(output);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].name, "feature/add-login");
        assert_eq!(entries[1].name, "fix-bug#123");
        assert_eq!(entries[2].name, "user@work");
    }

    #[test]
    fn test_parse_bookmark_entries_large_ahead_behind() {
        let output = r#"{"name":"feature","remote":"origin","change_id":"abc","synced":false,"ahead":1000,"behind":500}"#;
        let entries = parse_bookmark_entries(output);
        assert_eq!(entries[0].ahead, Some(1000));
        assert_eq!(entries[0].behind, Some(500));
    }

    #[test]
    fn test_compute_sync_state_zero_counts_not_synced() {
        // Edge case: synced=false but ahead=0, behind=0
        let local = BookmarkEntry {
            name: "feature".to_string(),
            remote: None,
            change_id: Some("abc".to_string()),
            synced: false,
            ahead: None,
            behind: None,
        };
        let remote = BookmarkEntry {
            name: "feature".to_string(),
            remote: Some("origin".to_string()),
            change_id: Some("abc".to_string()),
            synced: false, // Not synced flag
            ahead: Some(0),
            behind: Some(0),
        };

        let state = compute_sync_state(&local, Some(&remote));
        // Should treat 0/0 with synced=false as Synced
        assert!(matches!(state, BookmarkSyncState::Synced));
    }

    #[test]
    fn test_compute_sync_state_only_ahead() {
        let local = BookmarkEntry {
            name: "feature".to_string(),
            remote: None,
            change_id: Some("abc".to_string()),
            synced: false,
            ahead: None,
            behind: None,
        };
        let remote = BookmarkEntry {
            name: "feature".to_string(),
            remote: Some("origin".to_string()),
            change_id: Some("xyz".to_string()),
            synced: false,
            ahead: None, // No ahead info
            behind: Some(5),
        };

        let state = compute_sync_state(&local, Some(&remote));
        assert!(matches!(state, BookmarkSyncState::Ahead { count: 5 }));
    }

    #[test]
    fn test_compute_sync_state_only_behind() {
        let local = BookmarkEntry {
            name: "feature".to_string(),
            remote: None,
            change_id: Some("abc".to_string()),
            synced: false,
            ahead: None,
            behind: None,
        };
        let remote = BookmarkEntry {
            name: "feature".to_string(),
            remote: Some("origin".to_string()),
            change_id: Some("xyz".to_string()),
            synced: false,
            ahead: Some(3),
            behind: None, // No behind info
        };

        let state = compute_sync_state(&local, Some(&remote));
        assert!(matches!(state, BookmarkSyncState::Behind { count: 3 }));
    }

    #[test]
    fn test_compute_sync_state_large_divergence() {
        let local = BookmarkEntry {
            name: "feature".to_string(),
            remote: None,
            change_id: Some("abc".to_string()),
            synced: false,
            ahead: None,
            behind: None,
        };
        let remote = BookmarkEntry {
            name: "feature".to_string(),
            remote: Some("origin".to_string()),
            change_id: Some("xyz".to_string()),
            synced: false,
            ahead: Some(1000),
            behind: Some(500),
        };

        let state = compute_sync_state(&local, Some(&remote));
        if let BookmarkSyncState::Diverged { local_ahead, remote_ahead, .. } = state {
            assert_eq!(local_ahead, 500);
            assert_eq!(remote_ahead, 1000);
        } else {
            panic!("Expected Diverged state");
        }
    }

    #[test]
    fn test_deleted_bookmark_not_included() {
        // A deleted bookmark has remote=null and change_id=null
        // It should NOT be included in local_entries and should NOT match any change
        let output = r#"{"name":"deleted-feature","remote":null,"change_id":null,"synced":false,"ahead":null,"behind":null}
{"name":"deleted-feature","remote":"origin","change_id":"abc123","synced":false,"ahead":null,"behind":null}
{"name":"active-feature","remote":null,"change_id":"xyz789","synced":false,"ahead":null,"behind":null}"#;

        let entries = parse_bookmark_entries(output);
        assert_eq!(entries.len(), 3);

        // Filter like query_bookmarks does - deleted bookmarks have null change_id
        let local_entries: Vec<_> = entries
            .iter()
            .filter(|e| e.remote.is_none() && e.change_id.is_some())
            .collect();

        // Only the active bookmark should be included
        assert_eq!(local_entries.len(), 1);
        assert_eq!(local_entries[0].name, "active-feature");
        assert_eq!(local_entries[0].change_id, Some("xyz789".to_string()));
    }

    #[test]
    fn test_empty_change_id_does_not_match_all_changes() {
        // Regression test: empty string change_id would match any change via starts_with("")
        // This tests the matching logic at a unit level

        // Simulate what happens if a bookmark somehow has an empty change_id
        let bookmark_change_id = "";
        let change_id = "abc123def456";

        // The old code would match because "abc123def456".starts_with("") is true
        // The fix checks !bookmark_change_id.is_empty() first
        let matches = !bookmark_change_id.is_empty() && change_id.starts_with(bookmark_change_id);
        assert!(!matches, "Empty change_id should not match any change");
    }
}
