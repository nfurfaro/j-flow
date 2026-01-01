use anyhow::Result;
use crate::config::Config;
use crate::jj;
use crate::ui::{get_icon_set, get_theme, Renderer};

pub fn run(config: &Config, remote_override: Option<&str>) -> Result<()> {
    let theme = get_theme(&config.display.theme);
    let icons = get_icon_set(&config.display.icons);
    let renderer = Renderer::new(theme, icons);

    let remote = remote_override.unwrap_or(&config.remote.name);

    // Fetch from remote
    renderer.info(&format!("Fetching from {}...", remote));
    jj::run_jj(&["git", "fetch", "--remote", remote])?;

    // Rebase onto trunk
    let trunk_ref = config.trunk_ref();
    renderer.info(&format!("Rebasing stack onto {}...", trunk_ref));
    jj::run_jj(&["rebase", "-d", &trunk_ref])?;

    renderer.success("Successfully pulled and rebased!");
    println!();

    // Show updated stack
    let revset = config.stack_revset();
    let stack = jj::get_stack(&revset, &config.remote.name)?;
    renderer.render_stack(&stack, &config.trunk_ref());

    Ok(())
}
