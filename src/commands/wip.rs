use anyhow::Result;

use crate::config::Config;
use crate::jj;
use crate::ui::{get_icon_set, get_theme, Renderer};

/// Get the wip bookmark name for the current user
fn wip_bookmark_name() -> Result<String> {
    // Get username from jj config (user.name)
    let output = jj::run_jj(&["config", "get", "user.name"])?;
    let username = output.trim();

    // Slugify: lowercase, replace spaces/special chars with dashes
    let slug: String = username
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    Ok(format!("wip/{}", slug))
}

pub fn run(
    config: &Config,
    subcommand: Option<&str>,
    force: bool,
) -> Result<()> {
    let theme = get_theme(&config.display.theme);
    let icons = get_icon_set(&config.display.icons);
    let renderer = Renderer::new(theme, icons);

    match subcommand {
        None => run_status(config, &renderer),
        Some("push") => run_push(config, &renderer, force),
        Some("pull") => run_pull(config, &renderer),
        Some("clean") => run_clean(config, &renderer, force),
        Some(cmd) => {
            renderer.error(&format!("Unknown subcommand: {}", cmd));
            println!();
            println!("Usage:");
            println!("  jf wip              # show wip status");
            println!("  jf wip push         # push stack to wip branch");
            println!("  jf wip pull         # pull wip branch and rebase");
            println!("  jf wip clean        # delete wip branch");
            Ok(())
        }
    }
}

/// Show status of wip bookmark
fn run_status(config: &Config, renderer: &Renderer) -> Result<()> {
    let bookmark = wip_bookmark_name()?;
    let remote = &config.remote.name;

    // Check if wip bookmark exists on remote
    let remote_ref = format!("{}@{}", bookmark, remote);
    if !revision_exists(&remote_ref) {
        renderer.info(&format!("No wip branch found ({})", bookmark));
        println!("  Use `jf wip push` to push your stack");
        return Ok(());
    }

    // Get changes in the wip bookmark
    let main_ref = config.trunk_ref();
    let revset = format!("{}::({}) ~ ::{})", main_ref, remote_ref, main_ref);
    let changes = jj::query_changes(&revset)?;

    renderer.info(&format!("{} on {}:", bookmark, remote));
    if changes.is_empty() {
        println!("  (no changes)");
    } else {
        for change in &changes {
            let short_id = &change.change_id[..8.min(change.change_id.len())];
            let desc = if change.description.is_empty() {
                "(no description)".to_string()
            } else {
                change.description.clone()
            };
            println!("  ○ {}  {}", short_id, desc);
        }
    }

    Ok(())
}

/// Push stack to wip bookmark
fn run_push(config: &Config, renderer: &Renderer, force: bool) -> Result<()> {
    let bookmark = wip_bookmark_name()?;
    let remote = &config.remote.name;

    // Check if we have any changes to push
    let revset = config.stack_revset();
    let changes = jj::query_changes(&revset)?;

    if changes.is_empty() {
        renderer.info("No changes in stack to push");
        return Ok(());
    }

    // Fetch first to get accurate remote state
    renderer.info("Checking remote...");
    jj::run_jj(&["git", "fetch", "--remote", remote])?;

    // Check if wip bookmark already exists on remote
    let remote_ref = format!("{}@{}", bookmark, remote);
    let exists_on_remote = revision_exists(&remote_ref);

    if exists_on_remote && !force {
        renderer.error(&format!("{} already exists on {}", bookmark, remote));

        // Show what's there
        let main_ref = config.trunk_ref();
        let existing_revset = format!("{}::({}) ~ ::({})", main_ref, remote_ref, main_ref);
        let existing_changes = jj::query_changes(&existing_revset)?;

        if !existing_changes.is_empty() {
            println!();
            for change in &existing_changes {
                let short_id = &change.change_id[..8.min(change.change_id.len())];
                let desc = if change.description.is_empty() {
                    "(no description)".to_string()
                } else {
                    change.description.clone()
                };
                println!("  ○ {}  {}", short_id, desc);
            }
        }

        println!();
        println!("  Use `--force` to overwrite, or `jf wip pull` to fetch it first.");
        return Ok(());
    }

    renderer.info(&format!(
        "Pushing {} changes to {}...",
        changes.len(),
        bookmark
    ));

    let local_exists = bookmark_exists(&bookmark);

    // If bookmark exists on remote but not locally, track it first
    if exists_on_remote && !local_exists {
        jj::run_jj(&["bookmark", "track", &format!("{}@{}", bookmark, remote)])?;
    }

    // Push based on current state
    if exists_on_remote {
        // Remote exists and is tracked - set and push
        jj::run_jj(&["bookmark", "set", &bookmark, "-r", "@"])?;
        jj::run_jj(&["git", "push", "--bookmark", &bookmark])?;
    } else if local_exists {
        // Local exists but not on remote - delete local, use --named to create fresh
        jj::run_jj(&["bookmark", "delete", &bookmark])?;
        jj::run_jj(&["git", "push", "--named", &format!("{}=@", bookmark)])?;
    } else {
        // Neither exists - use --named to create and push
        jj::run_jj(&["git", "push", "--named", &format!("{}=@", bookmark)])?;
    }

    renderer.success("Done!");

    Ok(())
}

/// Pull wip bookmark and rebase onto main
fn run_pull(config: &Config, renderer: &Renderer) -> Result<()> {
    let bookmark = wip_bookmark_name()?;
    let remote = &config.remote.name;

    // Check for local changes first
    let revset = config.stack_revset();
    let local_changes = jj::query_changes(&revset)?;

    if !local_changes.is_empty() {
        renderer.error("You have local changes:");
        println!();
        for change in &local_changes {
            let short_id = &change.change_id[..8.min(change.change_id.len())];
            let desc = if change.description.is_empty() {
                "(no description)".to_string()
            } else {
                change.description.clone()
            };
            println!("  ○ {}  {}", short_id, desc);
        }
        println!();
        println!("  Clean up your local stack first, then try again.");
        return Ok(());
    }

    // Fetch from remote
    renderer.info("Fetching from origin...");
    jj::run_jj(&["git", "fetch"])?;

    // Check if wip bookmark exists on remote
    let remote_ref = format!("{}@{}", bookmark, remote);
    if !revision_exists(&remote_ref) {
        renderer.error(&format!("No wip branch found ({})", bookmark));
        return Ok(());
    }

    // Get changes from wip
    let main_ref = config.trunk_ref();
    let wip_revset = format!("{}::({}) ~ ::({})", main_ref, remote_ref, main_ref);
    let wip_changes = jj::query_changes(&wip_revset)?;

    if wip_changes.is_empty() {
        renderer.info("No changes in wip branch");
        return Ok(());
    }

    renderer.info(&format!("Found {} changes in {}", wip_changes.len(), bookmark));

    // Rebase wip changes onto main@origin
    // The changes are returned newest-first, so we need the last one (oldest) as the base
    // and rebase everything onto main
    renderer.info(&format!("Rebasing onto {}...", main_ref));

    // Rebase the entire wip branch onto main
    jj::run_jj(&["rebase", "-s", &remote_ref, "-d", &main_ref])?;

    // Move @ to the tip (which is now rebased)
    // After rebase, the bookmark still points to the rebased tip
    jj::run_jj(&["edit", &bookmark])?;

    renderer.success("Done!");

    // Show the stack
    println!();
    let stack = jj::get_stack(&config.stack_revset(), &config.remote.name)?;
    renderer.render_stack(&stack, &config.trunk_ref());

    Ok(())
}

/// Clean up wip bookmark
fn run_clean(config: &Config, renderer: &Renderer, force: bool) -> Result<()> {
    let bookmark = wip_bookmark_name()?;
    let remote = &config.remote.name;

    // Check if bookmark exists
    let remote_ref = format!("{}@{}", bookmark, remote);
    let local_exists = bookmark_exists(&bookmark);
    let remote_exists = revision_exists(&remote_ref);

    if !local_exists && !remote_exists {
        renderer.info(&format!("No wip branch found ({})", bookmark));
        return Ok(());
    }

    // Get changes in the wip bookmark
    let main_ref = config.trunk_ref();
    let wip_ref = if remote_exists { &remote_ref } else { &bookmark };
    let revset = format!("{}::({}) ~ ::({})", main_ref, wip_ref, main_ref);
    let changes = jj::query_changes(&revset)?;

    renderer.info(&format!("{} contains {} changes:", bookmark, changes.len()));

    // Check if changes have PRs (bookmarks other than wip)
    let mut all_have_prs = true;
    for change in &changes {
        let short_id = &change.change_id[..8.min(change.change_id.len())];
        let desc = if change.description.is_empty() {
            "(no description)".to_string()
        } else {
            change.description.clone()
        };

        // Check if this change has a non-wip bookmark (indicating a PR)
        let has_pr = has_non_wip_bookmark(&change.change_id);

        if has_pr {
            println!("  ○ {}  {} ✓", short_id, desc);
        } else {
            println!("  ○ {}  {}", short_id, desc);
            all_have_prs = false;
        }
    }

    if !all_have_prs && !force {
        println!();
        renderer.error("Cannot clean: some changes not in any PR");
        println!("  Hint: push PRs with `jf push`, or use `--force` to delete anyway");
        return Ok(());
    }

    // Delete local bookmark
    if local_exists {
        jj::run_jj(&["bookmark", "delete", &bookmark])?;
    }

    // Delete remote bookmark
    if remote_exists {
        jj::run_jj(&["git", "push", "--bookmark", &bookmark, "--delete"])?;
    }

    renderer.success(&format!("Deleted bookmark {} (local and remote)", bookmark));

    Ok(())
}

/// Check if a revision exists
fn revision_exists(rev: &str) -> bool {
    use std::process::Command;

    Command::new("jj")
        .args(["log", "-r", rev, "--limit", "1", "--no-graph", "-T", "''"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if a bookmark exists locally
fn bookmark_exists(bookmark: &str) -> bool {
    use std::process::Command;

    let output = Command::new("jj")
        .args(["bookmark", "list", "--all"])
        .output()
        .ok();

    if let Some(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout.lines().any(|line| line.starts_with(bookmark))
    } else {
        false
    }
}

/// Check if a change has any bookmark other than wip/*
fn has_non_wip_bookmark(change_id: &str) -> bool {
    use std::process::Command;

    let output = Command::new("jj")
        .args(["log", "-r", change_id, "--no-graph", "-T", "bookmarks"])
        .output()
        .ok();

    if let Some(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let bookmarks = stdout.trim();
        if bookmarks.is_empty() {
            return false;
        }
        // Check if any bookmark doesn't start with "wip/"
        bookmarks.split_whitespace().any(|b| !b.starts_with("wip/"))
    } else {
        false
    }
}
