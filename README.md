# jflow (jf) - Beautiful Workflow Tool for Jujutsu

A radically simple workflow tool for [Jujutsu](https://github.com/martinvonz/jj) that makes patch-stack development with GitHub beautiful and effortless.

## Philosophy

**Query, don't track.** jflow has zero state files—it queries jj directly using powerful revsets. Your stack is always `::@ ~ ::main@origin`. Simple.

**Four commands. That's it.**
- `jf status` - See your beautiful stack
- `jf pr` - Create bookmark + PR
- `jf sync` - Update all bookmarks
- `jf pull` - Fetch + rebase

## Installation

### Prerequisites

- [Jujutsu (jj)](https://github.com/martinvonz/jj) installed
- Rust toolchain (for building)

### Build from source

```bash
cargo install --path .
```

Or with just the binary name:

```bash
cargo build --release
cp target/release/jf ~/.local/bin/  # or wherever in your PATH
```

## Quick Start

```bash
# In your jj repository
cd my-project

# Initialize jflow (creates .jflow.toml)
jf init

# See your stack
jf status

# Create a PR for a change
jf pr abc1234 my-feature-name

# Update all bookmarks after making changes
jf sync

# Pull latest and rebase
jf pull
```

## Commands

### `jf init`

Initialize jflow in your repository. Creates `.jflow.toml` with smart defaults.

```bash
jf init                # Interactive configuration
jf init --defaults     # Skip prompts, use defaults
```

**What it does:**
1. Checks if you're in a jj repository
2. Detects your main branch (main, master, or trunk)
3. Detects your remote name
4. Checks if gh CLI is available
5. Creates `.jflow.toml` with detected settings

**Interactive mode:**
- Prompts for main branch (detected: main)
- Prompts for remote (detected: origin)
- Choose theme (catppuccin, nord, dracula, default)
- Choose icons (unicode or ascii)
- Set bookmark prefix (default: jf/)

**Example output:**
```
🎯 Initializing jflow...

📝 Configuration (press Enter to use detected/default values)

Main branch name [main]: 
Remote name [origin]: 

🎨 Available themes:
  1. catppuccin (warm pastels) [default]
  2. nord (cool arctic)
  3. dracula (high contrast)
  4. default (terminal colors)
Choose theme (1-4): 1

✨ Icon style:
  1. unicode (●○◆→) [default]
  2. ascii (*o#->)
Choose icons (1-2): 1

Bookmark prefix [jf/]: 

✓ Created .jflow.toml

📋 Configuration Summary:
  Stack revset: ::@ ~ ::main@origin
  Theme: catppuccin
  Icons: unicode
  Bookmark prefix: jf/

💡 Next steps:
  1. View your stack: jf status
  2. Create a PR: jf pr <change-id> <bookmark-name>
  3. Edit config: .jflow.toml
```

### `jf status`

Beautiful visualization of your stack with PR status.

```
╭─ Your Stack ────────────────────────────────╮
│                                              │
│  ●  qwer5678  Add login screen              │
│      💡 ready to create PR                   │
│  │                                           │
│  ○  tyui9012  Add backend API               │
│      → jf/add-backend-api                   │
│      ⏳ awaiting review                      │
│  │                                           │
│  ○  asdf1234  Add REST library              │
│      → jf/add-rest-library                  │
│      ✅ approved, ready to merge             │
│  │                                           │
│  ◆  main@origin                             │
│                                              │
╰──────────────────────────────────────────────╯

💡 Suggestions:
  • jf pr qwer5678 add-login-screen
```

**Icons:**
- `●` Working copy (@)
- `○` Change in stack
- `◆` Main branch
- `→` Has bookmark
- `💡` Ready for action

### `jf pr <change-id> <bookmark-name>`

Create a bookmark and PR for a specific change.

```bash
jf pr abc1234 add-rest-library
```

This:
1. Creates bookmark `jf/add-rest-library` at change `abc1234`
2. Pushes to GitHub
3. Creates PR with stack context (if `gh` CLI is available)

**Options:**
- `--title` - Custom PR title (defaults to commit description)

**With stack context enabled** (default), the PR description includes:
```markdown
Add REST library

---
**Part of stack:**
- ✅ **This PR** (Add REST library)
- ⏳ Add backend API (bookmark: `jf/add-backend-api`)
- ⏳ Add login screen (bookmark: `jf/add-login-screen`)
```

**Requirements:**
- `gh` CLI installed for automatic PR creation
- Without `gh`, bookmark is pushed but PR must be created manually

### `jf sync`

Update all bookmarks to their current commit positions and push.

```bash
jf sync
```

After rebasing or editing changes, bookmarks need to be updated to point to the new commits (remember: jj change IDs are stable, but commit IDs change). This command does it automatically for all bookmarks in your stack.

**What it does:**
1. Finds all bookmarks in your stack (`::@ ~ ::main@origin`)
2. For each bookmark, finds the current commit for that change ID
3. Updates the bookmark to point to the new commit
4. Pushes all bookmarks to remote

**Options:**
- `--dry-run` - Show what would be done without making changes

**Example output:**
```
ℹ Found 3 bookmark(s) to sync
  Updated jf/core-library → abc1234
  Updated jf/api-layer → def5678
  Updated jf/ui-component → ghi9012
ℹ Pushing bookmarks to remote...
✓ Successfully synced all bookmarks!
```

**When to use:**
- After `jj edit` on any change with a bookmark
- After `jj rebase` 
- After `jj squash` or other history modifications
- Before `jf pull` if you have local changes

### `jf pull`

Fetch from remote and rebase your stack.

```bash
jf pull
```

Equivalent to:
```bash
jj git fetch
jj rebase -d main@origin
```

## Configuration

Create `.jflow.toml` in your repository root:

```toml
[stack]
revset = "::@ ~ ::main@origin"
main_branch = "main"
remote = "origin"

[display]
theme = "catppuccin"  # catppuccin, nord, dracula, default
icons = "unicode"      # unicode or ascii

[bookmarks]
prefix = "jf/"
```

See [`.jflow.toml.example`](.jflow.toml.example) for all options.

## Themes

**Catppuccin Mocha** (default)
- Warm, pastel colors
- Excellent contrast

**Nord**
- Cool, arctic palette
- Easy on the eyes

**Dracula**
- High contrast
- Popular dark theme

**Default**
- Uses terminal colors
- Maximum compatibility

## How It Works

### Revset-Powered

jflow uses jj's revset language under the hood:

```rust
// Your stack
"::@ ~ ::main@origin"

// Changes with bookmarks
"bookmarks() & (::@ ~ ::main@origin)"

// Changes ready for PR
"(::@ ~ ::main@origin) ~ bookmarks()"
```

No metadata files. No state tracking. Just queries.

### GitHub Integration

**Via gh CLI (recommended):**
```toml
[github]
method = "gh-cli"
```

**Via API token:**
```toml
[github]
method = "api-token"
token = "ghp_..."
```

## Workflow Example

### Complete Patch Stack Workflow

```bash
# 0. Initialize jflow (first time only)
jf init

# 1. Start work (outside-in development)
jj new -m "Add REST library"
# ... implement library ...

jj new -m "Add backend API"  
# ... implement API using library ...

jj new -m "Add login screen"
# ... implement UI using API ...

# 2. View your stack
jf status

# Output:
# ╭─ Your Stack ────────────────────────────────╮
# │  ●  xyz789  Add login screen                │
# │      💡 ready to create PR                   │
# │  ○  def456  Add backend API                 │
# │      💡 ready to create PR                   │
# │  ○  abc123  Add REST library                │
# │      💡 ready to create PR                   │
# │  ◆  main@origin                             │
# ╰──────────────────────────────────────────────╯

# 3. Create PRs from bottom up (inside-out review)
jf pr abc123 rest-library
jf pr def456 backend-api  
jf pr xyz789 login-screen

# 4. Teammate reviews library PR and requests changes
# Edit the library commit directly
jj edit abc123
# ... make changes ...

# 5. Sync all bookmarks (library bookmark + all descendants)
jf sync

# Output:
# ℹ Found 3 bookmark(s) to sync
#   Updated jf/rest-library → abc123
#   Updated jf/backend-api → def456
#   Updated jf/login-screen → xyz789
# ℹ Pushing bookmarks to remote...
# ✓ Successfully synced all bookmarks!

# 6. All PRs automatically updated! 🎉
# The dependent PRs (API, UI) automatically rebased on the library fix

# 7. Library gets merged
# Pull and rebase
jf pull

# Output:
# ℹ Fetching from origin...
# ℹ Rebasing stack onto main@origin...
# ✓ Successfully pulled and rebased!
#
# Stack now shows:
# ╭─ Your Stack ────────────────────────────────╮
# │  ●  xyz789  Add login screen                │
# │  ○  def456  Add backend API                 │
# │  ◆  main@origin                             │
# ╰──────────────────────────────────────────────╯

# 8. Sync remaining PRs
jf sync
```

### Daily Workflow Commands

```bash
# Morning: Pull latest changes
jf pull

# Create new work
jj new -m "Feature X"

# Check status anytime
jf status

# Create PR when ready
jf pr <change-id> feature-x

# After making any edits
jf sync

# End of day: Push latest changes  
jf sync
```

## Development Status

✅ **All Commands Implemented!** ✅

Currently implemented:
- ✅ `jf init` - Initialize jflow with smart defaults
- ✅ `jf status` - Beautiful stack visualization
- ✅ `jf pr` - Create bookmark + PR (with gh CLI integration)
- ✅ `jf sync` - Update all bookmarks and push
- ✅ `jf pull` - Fetch + rebase stack

Ready to use for daily workflow!

## Contributing

This is an experimental project. Contributions welcome!

```bash
# Run with example
cd /path/to/your/jj/repo
jf status

# Build
cargo build

# Test
cargo test
```

## License

MIT

## Credits

Inspired by:
- [Jujutsu](https://github.com/martinvonz/jj) by Martin von Zweigbergk
- [Drew Deponte's patch stack methodology](https://drewdeponte.com/blog/how-we-should-be-using-git/)
- [Steve Klabnik's Jujutsu tutorial](https://steveklabnik.github.io/jujutsu-tutorial/)

Icons and colors from:
- [Catppuccin](https://github.com/catppuccin/catppuccin)
- [Nord](https://www.nordtheme.com/)
- [Dracula](https://draculatheme.com/)
