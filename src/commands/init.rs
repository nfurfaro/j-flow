use anyhow::{Context, Result};
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use crate::jj;
use crate::ui::{get_icon_set, get_theme, Renderer};

pub fn run(use_defaults: bool) -> Result<()> {
    let theme = get_theme("catppuccin");
    let icons = get_icon_set("unicode");
    let renderer = Renderer::new(theme, icons);

    // Check if we're in a jj repo
    jj::check_jj_available()?;
    if !is_jj_repo() {
        renderer.error("Not in a jj repository. Run 'jj git init' first.");
        return Ok(());
    }

    // Check if .jflow.toml already exists
    if Path::new(".jflow.toml").exists() {
        renderer.error(".jflow.toml already exists!");
        println!("To reconfigure, delete .jflow.toml and run 'jf init' again.");
        return Ok(());
    }

    println!("Initializing jflow...\n");

    // Detect repository settings
    let detected_trunk = detect_trunk_branch()?;
    let detected_remote = detect_default_remote()?;

    // Get configuration from user or use defaults
    let (trunk, remote, push_style, theme_choice, bookmark_prefix) = if use_defaults {
        renderer.info("Using default configuration");
        (
            detected_trunk.unwrap_or_else(|| "main".to_string()),
            detected_remote.unwrap_or_else(|| "origin".to_string()),
            "squash".to_string(),
            "catppuccin".to_string(),
            String::new(),
        )
    } else {
        get_interactive_config(detected_trunk, detected_remote)?
    };
    let icon_choice = "unicode".to_string();

    // Create .jflow.toml
    let config_content = create_config_content(&trunk, &remote, &push_style, &theme_choice, &icon_choice, &bookmark_prefix);

    fs::write(".jflow.toml", config_content).context("Failed to write .jflow.toml")?;

    renderer.success("Created .jflow.toml");
    println!();

    // Show summary
    print_summary(&trunk, &remote, &push_style, &theme_choice);

    // Show next steps
    println!("\n{} Next steps:", icons.lightbulb);
    println!("  1. View your stack: jf status");
    println!("  2. Push to GitHub: jf up");
    println!("  3. Edit config: .jflow.toml");
    println!();

    Ok(())
}

fn is_jj_repo() -> bool {
    jj::run_jj(&["status"]).is_ok()
}

fn detect_trunk_branch() -> Result<Option<String>> {
    // Try common branch names
    for branch in &["main", "master", "trunk"] {
        let remote_ref = format!("{}@origin", branch);
        if jj::run_jj(&["log", "-r", &remote_ref, "--limit", "1"]).is_ok() {
            return Ok(Some(branch.to_string()));
        }
    }
    Ok(None)
}

fn detect_default_remote() -> Result<Option<String>> {
    // Try to get remote list
    let output = jj::run_jj(&["git", "remote", "list"])?;

    // Parse output - format is "name url"
    for line in output.lines() {
        if let Some(remote_name) = line.split_whitespace().next() {
            return Ok(Some(remote_name.to_string()));
        }
    }

    Ok(None)
}

fn get_interactive_config(
    detected_trunk: Option<String>,
    detected_remote: Option<String>,
) -> Result<(String, String, String, String, String)> {
    println!("Configuration (press Enter to use detected/default values)\n");

    // Trunk branch
    let trunk_default = detected_trunk.unwrap_or_else(|| "main".to_string());
    let trunk = prompt("Trunk branch name", &trunk_default)?;

    // Remote
    let remote_default = detected_remote.unwrap_or_else(|| "origin".to_string());
    let remote = prompt("Remote name", &remote_default)?;

    // Push style
    println!("\nPush style:");
    println!("  1. squash (force-push updates) [default]");
    println!("  2. append (incremental commits, preserves review context)");
    let push_style = prompt_choice("Choose push style (1-2)", &["squash", "append"], "squash")?;

    // Theme
    println!("\nAvailable themes:");
    println!("  1. catppuccin (warm pastels) [default]");
    println!("  2. nord (cool arctic)");
    println!("  3. dracula (high contrast)");
    println!("  4. default (terminal colors)");
    let theme_choice = prompt_choice(
        "Choose theme (1-4)",
        &["catppuccin", "nord", "dracula", "default"],
        "catppuccin",
    )?;

    // Bookmark prefix
    let bookmark_prefix = prompt("Bookmark prefix (leave empty for none)", "")?;

    Ok((trunk, remote, push_style, theme_choice, bookmark_prefix))
}

fn prompt(question: &str, default: &str) -> Result<String> {
    if default.is_empty() {
        print!("{}: ", question);
    } else {
        print!("{} [{}]: ", question, default);
    }
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let trimmed = input.trim();
    if trimmed.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(trimmed.to_string())
    }
}

fn prompt_choice(question: &str, choices: &[&str], default: &str) -> Result<String> {
    print!("{}: ", question);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(default.to_string());
    }

    // Parse as number (1-indexed)
    if let Ok(num) = trimmed.parse::<usize>() {
        if num > 0 && num <= choices.len() {
            return Ok(choices[num - 1].to_string());
        }
    }

    // Or use as direct choice
    if choices.contains(&trimmed) {
        return Ok(trimmed.to_string());
    }

    // Invalid input, use default
    Ok(default.to_string())
}

fn create_config_content(
    trunk: &str,
    remote: &str,
    push_style: &str,
    theme: &str,
    icons: &str,
    bookmark_prefix: &str,
) -> String {
    format!(
        r#"# jflow configuration
# Generated by jf init

[remote]
# Remote name
name = "{}"

# Trunk branch name
trunk = "{}"

[github]
# Push style: "squash" (force-push) or "append" (incremental commits)
push_style = "{}"

# Merge style: "squash", "merge", or "rebase"
merge_style = "squash"

# Add stack context to PR descriptions
stack_context = true

[display]
# Color theme: catppuccin, nord, dracula, default
theme = "{}"

# Show git commit hashes alongside change IDs
show_commit_ids = false

# Icon style: unicode or ascii
icons = "{}"

[bookmarks]
# Prefix for bookmarks (e.g., "jf/" creates bookmarks like "jf/my-feature")
prefix = "{}"
"#,
        remote, trunk, push_style, theme, icons, bookmark_prefix
    )
}

fn print_summary(trunk: &str, remote: &str, push_style: &str, theme: &str) {
    println!("Configuration Summary:");
    println!("  Remote: {}", remote);
    println!("  Trunk: {}", trunk);
    println!("  Push style: {}", push_style);
    println!("  Theme: {}", theme);
}
