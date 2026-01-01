use anyhow::Result;
use crate::config::Config;
use crate::jj;
use crate::ui::{get_icon_set, get_theme, Renderer};

pub fn run(config: &Config) -> Result<()> {
    // Check jj is available
    jj::check_jj_available()?;

    // Get theme and icons
    let theme = get_theme(&config.display.theme);
    let icons = get_icon_set(&config.display.icons);
    let renderer = Renderer::new(theme, icons);

    // Query the stack
    let revset = config.stack_revset();
    let stack = jj::get_stack(&revset, &config.remote.name)?;

    // Render
    renderer.render_stack(&stack, &config.trunk_ref());

    Ok(())
}
