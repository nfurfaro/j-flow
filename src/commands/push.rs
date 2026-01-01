use anyhow::{Context, Result};
use std::io::{self, Write};
use std::process::Command;

use crate::config::Config;
use crate::jj;
use crate::ui::{get_icon_set, get_theme, Renderer};

pub fn run(
    config: &Config,
    revision: Option<&str>,
    bookmark: Option<&str>,
    force_squash: bool,
    force_append: bool,
    dry_run: bool,
) -> Result<()> {
    let theme = get_theme(&config.display.theme);
    let icons = get_icon_set(&config.display.icons);
    let renderer = Renderer::new(theme, icons);

    // Determine push style
    let push_style = if force_squash {
        "squash"
    } else if force_append {
        "append"
    } else {
        &config.github.push_style
    };

    // Get the changes to push
    let revset = revision.map(|r| r.to_string()).unwrap_or_else(|| config.stack_revset());
    let changes = jj::query_changes(&revset)?;

    if changes.is_empty() {
        renderer.info("No changes to push");
        return Ok(());
    }

    renderer.info(&format!("Found {} change(s) to push (style: {})", changes.len(), push_style));

    if dry_run {
        println!("\nDry run - would push:");
        for change in &changes {
            let short_id = &change.change_id[..8.min(change.change_id.len())];
            let desc = change.description.lines().next().unwrap_or("(no description)");
            let bookmark_info = if change.bookmarks.is_empty() {
                " [needs bookmark]".to_string()
            } else {
                format!(" [{}]", change.bookmarks.join(", "))
            };
            println!("  {} {}{}", short_id, desc, bookmark_info);
        }
        return Ok(());
    }

    // Check for empty descriptions
    let empty_desc_changes: Vec<_> = changes
        .iter()
        .filter(|c| c.description.trim().is_empty())
        .collect();

    if !empty_desc_changes.is_empty() {
        renderer.error("Cannot push changes without descriptions:");
        for change in &empty_desc_changes {
            let short_id = &change.change_id[..8.min(change.change_id.len())];
            println!("  {} (no description)", short_id);
        }
        println!();
        renderer.info("Add descriptions with: jj describe -r <change-id> -m \"Description\"");
        anyhow::bail!("Changes must have descriptions before pushing");
    }

    // Process each change
    for change in &changes {
        let short_id = &change.change_id[..8.min(change.change_id.len())];
        let desc = change.description.lines().next().unwrap_or("(no description)");

        // Check if change has a bookmark
        let change_bookmark = if !change.bookmarks.is_empty() {
            change.bookmarks[0].clone()
        } else if let Some(provided_bookmark) = bookmark {
            // Use provided bookmark (only makes sense for single change)
            let full_name = format!("{}{}", config.bookmarks.prefix, provided_bookmark);
            renderer.info(&format!("Creating bookmark '{}' at {}", full_name, short_id));
            jj::create_bookmark(&full_name, &change.change_id)?;
            full_name
        } else {
            // Prompt for bookmark name
            let bookmark_name = prompt_bookmark_name(short_id, desc)?;
            if bookmark_name.is_empty() {
                renderer.info(&format!("Skipping {} (no bookmark provided)", short_id));
                continue;
            }
            let full_name = format!("{}{}", config.bookmarks.prefix, bookmark_name);
            renderer.info(&format!("Creating bookmark '{}' at {}", full_name, short_id));
            jj::create_bookmark(&full_name, &change.change_id)?;
            full_name
        };

        // Push the bookmark
        renderer.info(&format!("Pushing {}...", change_bookmark));
        push_bookmark(&change_bookmark, &config.remote.name, push_style == "squash")?;

        // Check if PR exists, create if not
        if is_gh_available() {
            match get_pr_for_branch(&change_bookmark)? {
                Some(pr_url) => {
                    renderer.info(&format!("PR exists: {}", pr_url));
                }
                None => {
                    renderer.info("Creating pull request...");
                    let pr_title = desc;
                    let pr_body = if config.github.stack_context {
                        create_pr_body_with_stack(&change, config)?
                    } else {
                        change.description.clone()
                    };

                    // Determine base branch (parent's bookmark or trunk)
                    let base = get_base_branch_for_change(&change.change_id, config)?;
                    create_github_pr(&change_bookmark, &base, pr_title, &pr_body)?;
                    renderer.success("Pull request created!");
                }
            }
        }
    }

    renderer.success("Done!");
    Ok(())
}

fn prompt_bookmark_name(change_id: &str, description: &str) -> Result<String> {
    print!("Bookmark name for {} ({}) [skip]: ", change_id, description);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().to_string())
}

fn push_bookmark(bookmark: &str, remote: &str, _force: bool) -> Result<()> {
    // First, ensure the bookmark is tracked on the remote
    // This is needed for new bookmarks
    let track_ref = format!("{}@{}", bookmark, remote);
    let _ = jj::run_jj(&["bookmark", "track", &track_ref]);
    // Ignore errors - bookmark might already be tracked or not exist on remote yet

    // Push the bookmark
    let args = vec!["git", "push", "--bookmark", bookmark];
    jj::run_jj(&args)?;
    Ok(())
}

fn is_gh_available() -> bool {
    Command::new("gh")
        .arg("--version")
        .output()
        .is_ok()
}

fn get_pr_for_branch(branch: &str) -> Result<Option<String>> {
    let output = Command::new("gh")
        .args(["pr", "view", branch, "--json", "url", "-q", ".url"])
        .output()
        .context("Failed to check for existing PR")?;

    if output.status.success() {
        let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !url.is_empty() {
            return Ok(Some(url));
        }
    }
    Ok(None)
}

fn get_base_branch_for_change(change_id: &str, config: &Config) -> Result<String> {
    // Get parent of this change
    // Use short ID (first 8 chars) with `-` suffix for parent
    let short_id = &change_id[..8.min(change_id.len())];
    let parent_output = jj::run_jj(&[
        "log",
        "-r", &format!("{}-", short_id),
        "-T", "bookmarks",
        "--no-graph",
    ])?;

    // If parent has a bookmark, use it as base
    let parent_bookmark = parent_output.trim();
    if !parent_bookmark.is_empty() {
        // Parse first bookmark (they're space-separated)
        if let Some(bookmark) = parent_bookmark.split_whitespace().next() {
            // Filter out remote-tracking bookmarks
            if !bookmark.contains('@') {
                return Ok(bookmark.to_string());
            }
        }
    }

    // Otherwise use trunk
    Ok(config.remote.trunk.clone())
}

fn create_github_pr(branch: &str, base: &str, title: &str, body: &str) -> Result<()> {
    let output = Command::new("gh")
        .args([
            "pr", "create",
            "--head", branch,
            "--base", base,
            "--title", title,
            "--body", body,
        ])
        .output()
        .context("Failed to create PR with gh CLI")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("gh pr create failed: {}", stderr);
    }

    // Print gh output (contains PR URL)
    let stdout = String::from_utf8_lossy(&output.stdout);
    print!("{}", stdout);

    Ok(())
}

fn create_pr_body_with_stack(change: &jj::Change, config: &Config) -> Result<String> {
    let mut body = change.description.clone();

    // Add stack context
    body.push_str("\n\n---\n\n");
    body.push_str("**Part of stack:**\n\n");

    // Get stack to find related changes
    let revset = config.stack_revset();
    let stack = jj::get_stack(&revset, &config.remote.name)?;

    // Find this change's position in stack
    let mut found_current = false;
    for item in &stack {
        if item.change.change_id == change.change_id {
            found_current = true;
            body.push_str(&format!(
                "- **This PR** ({})\n",
                change.description.lines().next().unwrap_or("This change")
            ));
        } else if let Some(bookmark) = &item.bookmark {
            let status = if found_current { "⏳" } else { "✓" };
            body.push_str(&format!(
                "- {} {} (bookmark: `{}`)\n",
                status,
                item.change.description.lines().next().unwrap_or("Change"),
                bookmark
            ));
        }
    }

    Ok(body)
}
