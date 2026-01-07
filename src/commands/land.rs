use anyhow::{Context, Result};
use std::process::Command;

use crate::config::Config;
use crate::jj;
use crate::ui::{get_icon_set, get_theme, Renderer};

pub fn run(config: &Config, bookmark: Option<&str>, dry_run: bool) -> Result<()> {
    let theme = get_theme(&config.display.theme);
    let icons = get_icon_set(&config.display.icons);
    let renderer = Renderer::new(theme, icons);

    // Fetch latest from remote
    renderer.info(&format!("Fetching from {}...", config.remote.name));
    jj::run_jj(&["git", "fetch", "--remote", &config.remote.name])?;

    // Find merged bookmarks
    let merged_bookmarks = if let Some(b) = bookmark {
        // Check if specific bookmark is merged
        if is_pr_merged(b)? {
            vec![b.to_string()]
        } else {
            renderer.info(&format!("PR for '{}' is not merged yet", b));
            return Ok(());
        }
    } else {
        // Auto-detect merged PRs
        find_merged_bookmarks(config)?
    };

    if merged_bookmarks.is_empty() {
        renderer.info("No merged PRs found to clean up");
        return Ok(());
    }

    renderer.info(&format!("Found {} merged PR(s)", merged_bookmarks.len()));

    if dry_run {
        println!("\nDry run - would clean up:");
        for b in &merged_bookmarks {
            println!("  - {}", b);
        }
        return Ok(());
    }

    // Delete merged bookmarks
    for b in &merged_bookmarks {
        renderer.info(&format!("Deleting bookmark '{}'...", b));
        // Delete local bookmark
        if let Err(e) = jj::run_jj(&["bookmark", "delete", b]) {
            renderer.info(&format!("Note: Could not delete local bookmark: {}", e));
        }
    }

    // Rebase remaining stack onto trunk
    let trunk_ref = config.trunk_ref();
    renderer.info(&format!("Rebasing stack onto {}...", trunk_ref));
    if let Err(e) = jj::run_jj(&["rebase", "-d", &trunk_ref]) {
        renderer.info(&format!("Note: Rebase skipped or failed: {}", e));
    }

    renderer.success("Cleanup complete!");

    // Create a fresh empty commit for new work
    renderer.info("Creating fresh commit for new work...");
    jj::run_jj(&["new"])?;

    println!();

    // Show updated stack
    let revset = config.stack_revset();
    let stack = jj::get_stack(&revset, &config.remote.name)?;
    renderer.render_stack(&stack, &config.trunk_ref());

    Ok(())
}

fn is_pr_merged(bookmark: &str) -> Result<bool> {
    let output = Command::new("gh")
        .args(["pr", "view", bookmark, "--json", "state", "-q", ".state"])
        .output()
        .context("Failed to check PR state")?;

    if output.status.success() {
        let state = String::from_utf8_lossy(&output.stdout).trim().to_lowercase();
        return Ok(state == "merged");
    }
    Ok(false)
}

fn find_merged_bookmarks(config: &Config) -> Result<Vec<String>> {
    // Get all bookmarks in our stack
    let revset = format!("bookmarks() & ({})", config.stack_revset());
    let changes = jj::query_changes(&revset)?;

    let mut merged = Vec::new();

    for change in changes {
        for bookmark in &change.bookmarks {
            // Skip remote-tracking bookmarks
            if bookmark.contains('@') {
                continue;
            }

            // Check if this bookmark's PR is merged
            if is_pr_merged(bookmark).unwrap_or(false) {
                merged.push(bookmark.clone());
            }
        }
    }

    Ok(merged)
}
