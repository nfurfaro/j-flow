# jflow (jf) - Development Context

## Project Overview

**jflow** is a CLI tool (`jf`) that provides a beautiful, opinionated workflow for [Jujutsu](https://github.com/martinvonz/jj) (jj) VCS with GitHub integration. It implements a "stacked changes" workflow similar to Graphite or ghstack, but built specifically for jj.

## Key Concepts

### Stacked Changes Workflow
- Each logical change gets its own commit in the stack
- Changes are developed "outside-in" (base first, then dependent changes)
- PRs are reviewed "inside-out" (leaf changes first, then base)
- Each commit maps to one GitHub PR via bookmarks

### jj Terminology
- **Change**: A logical unit of work (has a stable change ID)
- **Commit**: A snapshot (has a commit ID that changes on amend)
- **Bookmark**: jj's equivalent of a git branch (points to a change)
- **Revset**: Query language for selecting commits (e.g., `::@ ~ ::main@origin`)

## Architecture

```
src/
├── main.rs           # CLI entry point (clap-based)
├── config.rs         # Configuration loading (.jflow.toml)
├── commands/         # Command implementations
│   ├── init.rs       # Initialize jflow config
│   ├── status.rs     # Show stack with PR status
│   ├── push.rs       # Push changes, create/update PRs
│   ├── pull.rs       # Fetch and rebase stack
│   ├── land.rs       # Clean up merged PRs
│   ├── reorder.rs    # Reorder changes in stack
│   └── wip.rs        # Work-in-progress management
├── jj/               # jj interaction layer
│   ├── mod.rs        # Module exports
│   ├── query.rs      # Query jj for changes, bookmarks
│   ├── types.rs      # Data structures (Change, BookmarkSyncState)
│   └── runner.rs     # CommandRunner trait for mocking
└── ui/               # Terminal UI
    ├── colors.rs     # Theme definitions (catppuccin, nord, etc.)
    ├── icons.rs      # Icon sets (unicode, ascii, nerdfont)
    └── render.rs     # Stack rendering
```

## Configuration

### Config Hierarchy
1. **Local**: `.jflow.toml` in repo (or parent directories)
2. **Global**: `~/.jflow.toml` (user defaults)
3. **Defaults**: Built-in fallbacks

Local config values override global config.

### Config Schema (.jflow.toml)
```toml
[remote]
name = "origin"           # Remote name
primary = "main"          # Primary branch (main/master/trunk)
                          # Note: "trunk" is accepted as alias for backward compat

[github]
push_style = "squash"     # "squash" (force-push) or "append" (incremental)
merge_style = "squash"    # "squash", "merge", or "rebase"
stack_context = true      # Add stack info to PR descriptions

[display]
theme = "catppuccin"      # catppuccin, nord, dracula, default
icons = "unicode"         # unicode, ascii, nerdfont
show_commit_ids = false   # Show git commit hashes

[bookmarks]
prefix = ""               # Prefix for auto-created bookmarks (e.g., "jf/")
```

## Commands

| Command | Description |
|---------|-------------|
| `jf` / `jf status` | Show stack with sync status |
| `jf init` | Initialize jflow config (skips if global exists) |
| `jf init --local` | Force create local .jflow.toml |
| `jf push` / `jf up` | Push changes, create PRs |
| `jf pull` / `jf down` | Fetch and rebase |
| `jf land` | Clean up merged PRs |
| `jf reorder` | Reorder stack changes |
| `jf wip` | Manage work-in-progress |

## Key Implementation Details

### Stack Query
The stack is queried using revset: `::@ ~ ::primary@remote`
- `::@` = all ancestors of working copy
- `~ ::primary@remote` = excluding ancestors of remote primary branch

### Bookmark Sync Detection
`query_bookmarks()` parses `jj bookmark list --all` with a template to get:
- Local bookmarks and their change IDs
- Remote tracking status (synced, ahead, behind, diverged)

### PR Workflow
1. `jf push` ensures primary branch exists on remote
2. Creates bookmarks for changes without them
3. Pushes bookmarks to remote
4. Creates GitHub PRs via `gh` CLI (if available)

### Landing PRs
`jf land` workflow:
1. Fetches latest from remote
2. Finds bookmarks whose PRs are merged (via `gh pr view --json state`)
3. Deletes local and remote bookmarks
4. Rebases remaining stack onto primary
5. Creates a new empty commit for continued work

## Testing

### Test Structure
- **Unit tests**: In each module (`#[cfg(test)]` blocks)
- **Integration tests**: `tests/integration_test.rs`

### Running Tests
```bash
cargo test                    # All tests
cargo test config             # Config tests only
cargo test --test integration # Integration tests only
```

### Test Helpers
- `create_jj_repo()` - Creates temp jj repo
- `create_jj_repo_with_remote()` - Creates repo with local bare git as "origin"
- `create_jflow_config()` - Writes test .jflow.toml

### Mocking
`CommandRunner` trait in `jj/runner.rs` allows mocking jj commands:
```rust
pub trait CommandRunner: Send + Sync {
    fn run(&self, program: &str, args: &[&str]) -> Result<String>;
}
```

## Dependencies

### Runtime
- `clap` - CLI parsing
- `serde` / `toml` / `serde_json` - Config and jj output parsing
- `colored` / `console` - Terminal output
- `anyhow` / `thiserror` - Error handling
- `dirs` - Home directory lookup

### External Tools
- `jj` - Jujutsu VCS (required)
- `gh` - GitHub CLI (optional, for PR operations)

## Common Development Tasks

### Adding a New Command
1. Create `src/commands/newcmd.rs`
2. Add to `src/commands/mod.rs`
3. Add variant to `Commands` enum in `main.rs`
4. Add match arm in `main()` function

### Modifying Config
1. Update struct in `config.rs`
2. Add default function if needed
3. Update `merge()` function for global/local merging
4. Update tests
5. Update `init.rs` config template

### Adding Tests
- Unit tests: Add `#[test]` functions in module's `tests` block
- Integration tests: Add to `tests/integration_test.rs`
- Edge cases: Follow existing patterns for unicode, special chars, etc.

## Known Issues / Technical Debt

1. **Unused code warnings**: `CommandRunner` trait and some fields are defined but not yet used in production code (prepared for future mocking)

2. **Boolean merge behavior**: In config merging, booleans can't distinguish "not set" from "set to default" - overlay always wins

## Related Resources

- **jflow-book**: Documentation at `/Users/nick/dev/jflow-workspace/jflow-book/`
- **jj docs**: https://martinvonz.github.io/jj/
- **Stacked changes doc**: `stacked-changes-vanilla-jj.md` in workspace root
