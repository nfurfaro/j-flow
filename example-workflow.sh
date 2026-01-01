#!/usr/bin/env bash
# Example workflow using jflow

set -e

echo "🎯 jflow Example Workflow"
echo "=========================="
echo ""

# Ensure we're in a jj repo
if ! jj log -r @ &>/dev/null; then
    echo "❌ Not in a jj repository"
    exit 1
fi

echo "🎬 Step 0: Initialize jflow (first time only)"
if [ ! -f .jflow.toml ]; then
    echo "$ jf init --defaults"
    jf init --defaults
else
    echo "ℹ .jflow.toml already exists, skipping init"
fi

echo ""
echo "📊 Step 1: View your current stack"
echo "$ jf status"
jf status

echo ""
echo "📝 Step 2: Create some test changes"
echo "$ jj new -m 'Add core library'"
jj new -m "Add core library"
echo "$ jj new -m 'Add API layer'"
jj new -m "Add API layer"
echo "$ jj new -m 'Add UI component'"
jj new -m "Add UI component"

echo ""
echo "📊 View stack again"
echo "$ jf status"
jf status

echo ""
echo "🚀 Step 3: Create PRs for changes (inside-out)"
echo "Note: You'll need to replace the change IDs with actual ones from your stack"
echo ""
echo "Example commands (don't run - change IDs won't match):"
echo "  jf pr abc1234 core-library"
echo "  jf pr def5678 api-layer"
echo "  jf pr ghi9012 ui-component"

echo ""
echo "🔄 Step 4: After making changes, sync bookmarks"
echo "$ jf sync --dry-run"
jf sync --dry-run

echo ""
echo "📥 Step 5: Pull latest changes and rebase"
echo "$ jf pull"
echo "(This will fetch and rebase your stack)"

echo ""
echo "✅ Workflow complete!"
echo ""
echo "📚 Full command reference:"
echo "  jf status              - View your beautiful stack"
echo "  jf pr <id> <name>      - Create bookmark + PR"
echo "  jf sync                - Update all bookmarks"
echo "  jf pull                - Fetch + rebase"
echo ""
echo "💡 Pro tips:"
echo "  - Create PRs from bottom (core) to top (UI)"
echo "  - Use 'jf sync' after editing any change"
echo "  - Use 'jf pull' to stay up to date"
