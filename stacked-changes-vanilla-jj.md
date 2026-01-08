# Stacked Changes Workflow with Vanilla jj

The "outside-in development, inside-out review" methodology works well with jj, though it requires more manual coordination than jflow provides.

## Core Concept

Build features in dependency order (foundational changes first), but get them reviewed and merged starting from the bottom of the stack. Each change builds on the previous one.

## Creating a Stack

```bash
# Start from main
jj new main -m "Add REST client library"
# ... make changes ...

jj new -m "Add API layer using REST client"
# ... make changes ...

jj new -m "Add UI components using API"
# ... make changes ...
```

Your stack now looks like:
```
@  UI components      <- you are here
○  API layer
○  REST client
◆  main
```

## Pushing for Review

Each change needs a bookmark to create a PR:

```bash
# Create bookmarks for each change
jj bookmark create rest-client -r <change-id-1>
jj bookmark create api-layer -r <change-id-2>
jj bookmark create ui-components -r <change-id-3>

# Push all bookmarks
jj git push --bookmark rest-client --bookmark api-layer --bookmark ui-components
```

Then create PRs manually on GitHub, or use `gh`:

```bash
gh pr create --head rest-client --base main --title "Add REST client library"
gh pr create --head api-layer --base rest-client --title "Add API layer"
gh pr create --head ui-components --base api-layer --title "Add UI components"
```

Note: Each PR targets its parent branch, not main. This shows only that PR's changes in the diff.

## Editing a Change Mid-Stack

```bash
# Edit the API layer change
jj edit <api-layer-change-id>

# Make your fixes...

# Return to the top of your stack
jj edit <ui-components-change-id>
# Or create a new change on top
jj new <ui-components-change-id>
```

jj automatically rebases descendants when you edit, so UI components will include your API layer fixes.

## Pushing Updates

After editing, the bookmarks still point to old commits. Update them:

```bash
# Move bookmarks to current change positions
jj bookmark set rest-client -r <change-id-1>
jj bookmark set api-layer -r <change-id-2>
jj bookmark set ui-components -r <change-id-3>

# Force push (commits changed)
jj git push --bookmark rest-client --bookmark api-layer --bookmark ui-components
```

## After a PR Merges (Inside-Out)

When `rest-client` merges:

```bash
# Fetch latest
jj git fetch

# Delete the landed bookmark
jj bookmark delete rest-client

# Rebase your stack onto updated main
jj rebase -s <api-layer-change-id> -d main@origin

# Update the api-layer PR to target main instead of rest-client
gh pr edit api-layer --base main
```

Repeat as each PR merges from the bottom up.

## Reordering Changes

If you realize changes should be in a different order:

```bash
# Move a change to a different parent
jj rebase -r <change-to-move> -d <new-parent>

# Or rebase a change and all its descendants
jj rebase -s <change> -d <new-parent>
```

## Viewing Your Stack

```bash
# See your changes above main
jj log -r "::@ ~ ::main@origin"
```

## The Pain Points (What jflow Solves)

1. **Bookmark management** - You must manually create, set, and track bookmarks for each change
2. **PR targeting** - Each PR must target its parent branch; you must update this when parents merge
3. **Stack context** - PRs don't show where they sit in the stack; reviewers lack context
4. **Sync state** - No easy way to see which changes are pushed, ahead, or behind
5. **Landing** - Manual process to detect merged PRs, delete bookmarks, rebase, and retarget remaining PRs

The workflow is powerful, but the bookkeeping overhead is significant. That's the gap jflow fills.
