use anyhow::Result;

use crate::config::Config;
use crate::jj;
use crate::ui::{get_icon_set, get_theme, Renderer};

pub fn run(
    config: &Config,
    changes: Vec<String>,
    invert: bool,
    revision: Option<&str>,
) -> Result<()> {
    let theme = get_theme();
    let icons = get_icon_set();
    let renderer = Renderer::new(theme, icons);

    if invert {
        run_invert(config, &renderer, revision)
    } else if !changes.is_empty() {
        run_explicit(config, &renderer, changes, revision)
    } else {
        renderer.error("Specify changes to reorder, or use --invert");
        println!();
        println!("Usage:");
        println!("  jf reorder <change1> <change2> ...    # reorder in given order");
        println!("  jf reorder -f <c1> <c2> <c3> ...      # reorder starting from c1 (inclusive)");
        println!("  jf reorder --invert                   # reverse the stack");
        println!("  jf reorder --invert -f <change>       # reverse from change to @ (inclusive)");
        Ok(())
    }
}

/// Reorder changes in the explicit order given
/// Example: jf reorder abc def ghi
/// Results in: parent(abc) -> abc -> def -> ghi
/// With --from: jf reorder --from xyz abc def ghi
/// Results in: parent(xyz) -> xyz -> abc -> def -> ghi (--from is inclusive)
fn run_explicit(config: &Config, renderer: &Renderer, changes: Vec<String>, from: Option<&str>) -> Result<()> {
    // Build the full list of changes to reorder (--from is inclusive)
    let all_changes: Vec<String> = if let Some(from_change) = from {
        let mut v = vec![from_change.to_string()];
        v.extend(changes);
        v
    } else {
        changes
    };

    if all_changes.is_empty() {
        renderer.error("Need at least 1 change to reorder");
        return Ok(());
    }

    if all_changes.len() < 2 {
        renderer.error("Need at least 2 changes to reorder");
        return Ok(());
    }

    renderer.info(&format!("Reordering {} changes...", all_changes.len()));

    // Get the base (parent of the first change)
    let first_change = &all_changes[0];
    let base = get_parent(first_change)?;

    // Rebase each change onto the previous one
    let mut current_base = base;
    let mut last_change = String::new();
    for change in &all_changes {
        renderer.info(&format!("  Moving {} onto {}", change, short_id(&current_base)));
        jj::run_jj(&["rebase", "-r", change, "-d", &current_base])?;
        current_base = change.clone();
        last_change = change.clone();
    }

    // Move @ to the last reordered change so the stack displays correctly
    if !last_change.is_empty() {
        jj::run_jj(&["edit", &last_change])?;
    }

    renderer.success("Reorder complete!");
    println!();

    // Show updated stack
    let revset = config.stack_revset();
    let stack = jj::get_stack(&revset, &config.remote.name)?;
    renderer.render_stack(&stack, &config.main_branch_ref());

    Ok(())
}

/// Invert the stack (reverse order)
/// With -r, inverts from that change to @
/// Without -r, inverts the entire stack
fn run_invert(config: &Config, renderer: &Renderer, revision: Option<&str>) -> Result<()> {
    // Get the stack to invert
    let revset = if let Some(rev) = revision {
        format!("{}::@", rev)
    } else {
        config.stack_revset()
    };

    let changes = jj::query_changes(&revset)?;

    if changes.len() < 2 {
        renderer.info("Stack has fewer than 2 changes, nothing to invert");
        return Ok(());
    }

    renderer.info(&format!("Inverting {} changes...", changes.len()));

    // Changes come in reverse order (newest first), so we need to reverse them
    // to get oldest first, then that becomes our target order (which will invert the stack)
    let change_ids: Vec<String> = changes.iter().map(|c| c.change_id.clone()).collect();

    // Get the base (parent of the oldest change in the range)
    let oldest_change = &change_ids[change_ids.len() - 1];
    let base = get_parent(&short_id(oldest_change))?;

    // Rebase in reverse order: newest becomes first (on base), oldest becomes last
    let mut current_base = base;
    let mut last_change = String::new();
    for change_id in &change_ids {
        let short = short_id(change_id);
        renderer.info(&format!("  Moving {} onto {}", short, short_id(&current_base)));
        jj::run_jj(&["rebase", "-r", &short, "-d", &current_base])?;
        current_base = short.clone();
        last_change = short;
    }

    // Move @ to the new tip so the stack displays correctly
    if !last_change.is_empty() {
        jj::run_jj(&["edit", &last_change])?;
    }

    renderer.success("Stack inverted!");
    println!();

    // Show updated stack
    let stack_revset = config.stack_revset();
    let stack = jj::get_stack(&stack_revset, &config.remote.name)?;
    renderer.render_stack(&stack, &config.main_branch_ref());

    Ok(())
}

/// Get the parent of a change
fn get_parent(change: &str) -> Result<String> {
    let output = jj::run_jj(&["log", "-r", &format!("{}-", change), "-T", "change_id", "--no-graph", "--limit", "1"])?;
    Ok(output.trim().to_string())
}

/// Get a short ID (first 8 chars)
fn short_id(id: &str) -> String {
    id[..8.min(id.len())].to_string()
}
