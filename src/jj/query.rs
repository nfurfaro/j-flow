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
struct BookmarkEntry {
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
    // Use tracked() to check if this is a tracked remote ref before accessing tracking counts
    let template = r#"concat(
        "{\"name\":\"", name, "\",",
        "\"remote\":", if(remote, concat("\"", remote, "\""), "null"), ",",
        "\"change_id\":", if(normal_target, concat("\"", normal_target.change_id().short(), "\""), "null"), ",",
        "\"synced\":", synced, ",",
        "\"ahead\":", if(tracked(), tracking_ahead_count.exact(), "null"), ",",
        "\"behind\":", if(tracked(), tracking_behind_count.exact(), "null"),
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

    // Get local bookmarks (remote is null)
    let local_entries: Vec<_> = entries.iter().filter(|e| e.remote.is_none()).collect();

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
    // Match by prefix
    let mut result = Vec::new();
    for change in changes {
        let matched_bookmark = bookmarks
            .iter()
            .find(|b| change.change_id.starts_with(&b.change_id));

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
