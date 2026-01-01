use colored::Colorize;
use crate::jj::types::{BookmarkSyncState, ChangeWithStatus};
use super::{IconSet, Theme};

pub struct Renderer {
    theme: &'static Theme,
    icons: &'static IconSet,
}

impl Renderer {
    pub fn new(theme: &'static Theme, icons: &'static IconSet) -> Self {
        Self { theme, icons }
    }
    
    /// Render the stack status
    pub fn render_stack(&self, changes: &[ChangeWithStatus], main_ref: &str) {
        let total = changes.len();

        println!();
        let title = if total > 0 {
            format!("Your Stack ({} commits)", total)
        } else {
            "Your Stack".to_string()
        };
        self.print_box_top(&title);
        println!();

        if changes.is_empty() {
            println!("  No changes in stack");
            println!("  (All work is integrated into {})", main_ref);
        } else {
            for (i, item) in changes.iter().enumerate() {
                // Position: 1 is closest to trunk, total is the head
                let position = total - i;
                self.render_change(item, position, total);

                // Add spacing between changes (except for last)
                if i < changes.len() - 1 {
                    self.print_connection();
                }
            }
        }

        // Print main branch
        if !changes.is_empty() {
            self.print_connection();
        }
        self.print_main(main_ref);

        println!();
        self.print_box_bottom();
        println!();

        // Print suggestions
        self.print_suggestions(changes);
    }
    
    fn render_change(&self, item: &ChangeWithStatus, position: usize, total: usize) {
        let is_working = item.is_working;

        // Icon
        let icon = if is_working {
            self.icons.working
        } else {
            self.icons.change
        };

        let icon_colored = if is_working {
            icon.color(self.theme.mauve)
        } else {
            icon.color(self.theme.text)
        };

        // Position marker (e.g., "3/5")
        let position_marker = format!("{}/{}", position, total).color(self.theme.overlay);

        // Change ID (first 8 chars)
        let change_id = &item.change.change_id[..8.min(item.change.change_id.len())];
        let change_id_colored = change_id.color(self.theme.blue);

        // Description
        let description = item.change.description
            .lines()
            .next()
            .unwrap_or("(no description)")
            .color(self.theme.text);

        // Main line with position
        println!(
            "  {} {}  {}  {}",
            position_marker, icon_colored, change_id_colored, description
        );
        
        // Bookmark line with sync state (if exists)
        if let Some(bookmark) = &item.bookmark {
            self.render_sync_state(bookmark, &item.sync_state);
        }
        
        // Status line (aligned with bookmark line)
        if let Some(status_msg) = self.format_status(item) {
            println!("         {}", status_msg);
        }
    }
    
    /// Render bookmark with sync state visualization
    fn render_sync_state(&self, bookmark: &str, sync_state: &BookmarkSyncState) {
        let bookmark_icon = self.icons.bookmark.color(self.theme.teal);
        let bookmark_name = bookmark.color(self.theme.teal);

        match sync_state {
            BookmarkSyncState::NoBookmark => {
                // Shouldn't happen since we're called with a bookmark
            }
            BookmarkSyncState::LocalOnly => {
                println!(
                    "         {} {} {}",
                    bookmark_icon,
                    bookmark_name,
                    "(local only)".color(self.theme.overlay)
                );
            }
            BookmarkSyncState::Synced => {
                println!(
                    "         {} {} {}",
                    bookmark_icon,
                    bookmark_name,
                    "✓".color(self.theme.green)
                );
            }
            BookmarkSyncState::Ahead { count } => {
                // Local is ahead of remote
                println!(
                    "         {} {} {} {}",
                    bookmark_icon,
                    bookmark_name,
                    format!("↑{}", count).color(self.theme.green),
                    "ahead".color(self.theme.overlay)
                );
            }
            BookmarkSyncState::Behind { count } => {
                // Local is behind remote
                println!(
                    "         {} {} {} {}",
                    bookmark_icon,
                    bookmark_name,
                    format!("↓{}", count).color(self.theme.yellow),
                    "behind".color(self.theme.overlay)
                );
            }
            BookmarkSyncState::Diverged { local_ahead, remote_ahead, fork_point } => {
                // Show diverged state with fork visualization
                let fork_id = fork_point.as_deref().unwrap_or("???");

                // Fork visualization - the ○ fork point must align with ╭ and ╰:
                //                   ╭──●──●    local (+2)
                //       bookmark ───○ abc123
                //                   ╰──○──○    origin (+1) ⚠ diverged

                // Base indent for bookmark line (9 spaces to align with change_id)
                let base_indent = "         ";

                // Build the bookmark prefix: "{base_indent}{bookmark_icon} {bookmark_name} ───"
                let prefix = format!("{}{} {} ───", base_indent, self.icons.bookmark, bookmark);
                let prefix_width = console::measure_text_width(&prefix);

                // Fork arms (╭ and ╰) start at same column as the ○
                let fork_indent = " ".repeat(prefix_width);

                // Build chains: ──●──●──● for local, ──○──○──○ for remote
                let local_chain_dots: Vec<&str> = (0..*local_ahead).map(|_| "●").collect();
                let local_chain_str = local_chain_dots.join("──");
                let local_chain = format!("╭──{}    local (+{})", local_chain_str, local_ahead);
                println!(
                    "{}{}",
                    fork_indent,
                    local_chain.color(self.theme.green)
                );

                // Fork point with bookmark
                println!(
                    "{}○ {}",
                    prefix.color(self.theme.teal),
                    fork_id.color(self.theme.overlay)
                );

                // Remote branch (below fork point)
                let remote_chain_dots: Vec<&str> = (0..*remote_ahead).map(|_| "○").collect();
                let remote_chain_str = remote_chain_dots.join("──");
                let remote_chain = format!("╰──{}    origin (+{}) ⚠ diverged", remote_chain_str, remote_ahead);
                println!(
                    "{}{}",
                    fork_indent,
                    remote_chain.color(self.theme.red)
                );
            }
        }
    }

    fn format_status(&self, item: &ChangeWithStatus) -> Option<String> {
        if item.bookmark.is_none() && !item.is_working {
            Some(format!("{} ready to create PR", self.icons.lightbulb))
        } else {
            None
        }
    }
    
    fn print_connection(&self) {
        // Align pipe with the icon position
        // Main line: "  {pos} {icon}  {id}  {desc}"
        // "  1/1 " = 6 chars, then icon
        println!("      {}", self.icons.pipe.color(self.theme.overlay));
    }
    
    fn print_main(&self, main_ref: &str) {
        // Align with the icon position
        // Main line: "  {pos} {icon}  {id}  {desc}"
        // "  1/1 " = 6 chars, then icon
        println!(
            "      {}  {}",
            self.icons.main.color(self.theme.blue),
            main_ref.color(self.theme.blue)
        );
    }
    
    fn print_box_top(&self, title: &str) {
        let title_with_padding = format!(" {} ", title);
        let width: usize = 60;
        let title_len = console::measure_text_width(&title_with_padding);
        let remaining = width.saturating_sub(title_len + 2);
        let left_padding = remaining / 2;
        let right_padding = remaining - left_padding;
        
        println!(
            "╭{}{}{}╮",
            "─".repeat(left_padding),
            title_with_padding.color(self.theme.text),
            "─".repeat(right_padding)
        );
    }
    
    fn print_box_bottom(&self) {
        println!("╰{}╯", "─".repeat(60));
    }
    
    fn print_suggestions(&self, changes: &[ChangeWithStatus]) {
        let mut suggestions = Vec::new();

        // Check if there are changes without bookmarks
        let needs_bookmark = changes.iter().any(|c| c.bookmark.is_none() && !c.is_working);
        if needs_bookmark {
            suggestions.push(format!(
                "  {} Push to GitHub: jf push",
                self.icons.lightbulb
            ));
        }

        // Suggest pulling
        suggestions.push(format!(
            "  {} Update from remote: jf pull",
            self.icons.info
        ));

        if !suggestions.is_empty() {
            println!("{} Quick commands:", self.icons.lightbulb);
            for suggestion in suggestions {
                println!("{}", suggestion);
            }
            println!();
        }
    }
    
    /// Render error message
    pub fn error(&self, message: &str) {
        eprintln!(
            "{} {}",
            self.icons.error.color(self.theme.red),
            message.color(self.theme.red)
        );
    }
    
    /// Render success message
    pub fn success(&self, message: &str) {
        println!(
            "{} {}",
            self.icons.pr_approved.color(self.theme.green),
            message.color(self.theme.green)
        );
    }
    
    /// Render info message
    pub fn info(&self, message: &str) {
        println!(
            "{} {}",
            self.icons.info.color(self.theme.blue),
            message
        );
    }
}
