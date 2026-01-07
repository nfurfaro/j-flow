use anyhow::{Context, Result};
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use crate::jj;
use crate::ui::{get_icon_set, get_theme, Renderer};

pub fn run(use_defaults: bool, create_github_repo: bool) -> Result<()> {
    let theme = get_theme("default");
    let icons = get_icon_set("unicode");
    let renderer = Renderer::new(theme, icons);

    // Check if we're in a jj repo
    jj::check_jj_available()?;
    if !is_jj_repo() {
        renderer.error("Not in a jj repository. Run 'jj git init' first.");
        return Ok(());
    }

    // Create GitHub repo if requested
    if create_github_repo {
        create_github_repository(&renderer)?;
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
    let (trunk, remote, push_style, bookmark_prefix) = if use_defaults {
        renderer.info("Using default configuration");
        (
            detected_trunk.unwrap_or_else(|| "main".to_string()),
            detected_remote.unwrap_or_else(|| "origin".to_string()),
            "squash".to_string(),
            String::new(),
        )
    } else {
        get_interactive_config(detected_trunk, detected_remote)?
    };

    // Create .jflow.toml
    let config_content = create_config_content(&trunk, &remote, &push_style, &bookmark_prefix);

    fs::write(".jflow.toml", config_content).context("Failed to write .jflow.toml")?;

    renderer.success("Created .jflow.toml");
    println!();

    // Show summary
    print_summary(&trunk, &remote, &push_style);

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
) -> Result<(String, String, String, String)> {
    println!("Configuration (press Enter to use detected/default values)\n");

    // Trunk branch
    let trunk_default = detected_trunk.unwrap_or_else(|| "main".to_string());
    let trunk = prompt("Main branch name", &trunk_default)?;

    // Remote
    let remote_default = detected_remote.unwrap_or_else(|| "origin".to_string());
    let remote = prompt("Remote name", &remote_default)?;

    // Push style
    println!("\nPush style:");
    println!("  1. squash (force-push updates) [default]");
    println!("  2. append (incremental commits, preserves review context)");
    let push_style = prompt_choice("Choose push style (1-2)", &["squash", "append"], "squash")?;

    // Bookmark prefix
    let bookmark_prefix = prompt("\nBookmark prefix (leave empty for none)", "")?;

    Ok((trunk, remote, push_style, bookmark_prefix))
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
    bookmark_prefix: &str,
) -> String {
    format!(
        r#"# jflow configuration
# Generated by jf init

[remote]
# Remote name
name = "{}"

# Main branch name
trunk = "{}"

[github]
# Push style: "squash" (force-push) or "append" (incremental commits)
push_style = "{}"

# Merge style: "squash", "merge", or "rebase"
merge_style = "squash"

# Add stack context to PR descriptions
stack_context = true

[bookmarks]
# Prefix for bookmarks (e.g., "jf/" creates bookmarks like "jf/my-feature")
prefix = "{}"
"#,
        remote, trunk, push_style, bookmark_prefix
    )
}

fn print_summary(trunk: &str, remote: &str, push_style: &str) {
    println!("Configuration Summary:");
    println!("  Remote: {}", remote);
    println!("  Main branch: {}", trunk);
    println!("  Push style: {}", push_style);
}

fn create_github_repository(renderer: &Renderer) -> Result<()> {
    use std::process::Command;

    // Check if gh is available
    if Command::new("gh").arg("--version").output().is_err() {
        renderer.error("gh CLI not found. Install it from https://cli.github.com/");
        return Ok(());
    }

    // Check if remote already exists
    if detect_default_remote()?.is_some() {
        renderer.info("Remote already configured, skipping GitHub repo creation");
        return Ok(());
    }

    // Get repo name from current directory
    let current_dir = std::env::current_dir()?;
    let repo_name = current_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("repo");

    renderer.info(&format!("Creating GitHub repository '{}'...", repo_name));

    // Create repo with gh CLI (private by default, with source set to current dir)
    let output = Command::new("gh")
        .args(["repo", "create", repo_name, "--private", "--source", ".", "--remote", "origin"])
        .output()?;

    if output.status.success() {
        renderer.success("GitHub repository created and remote added");

        // Push main branch to set up tracking
        renderer.info("Pushing main branch...");
        let push_output = Command::new("jj")
            .args(["git", "push", "--named", "main=@-"])
            .output()?;

        if push_output.status.success() {
            renderer.success("Main branch pushed to origin");
        } else {
            // Try alternative: push current commit as main
            let _ = Command::new("git")
                .args(["push", "-u", "origin", "HEAD:main"])
                .output();
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        renderer.error(&format!("Failed to create GitHub repo: {}", stderr.trim()));
    }

    Ok(())
}
