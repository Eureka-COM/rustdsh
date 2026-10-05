#!/bin/sh
# Install recommended dsh plugins into a profile (idempotent per package).
# Also installs the bundled rdsh plugins (rdsh settings UI) so they are
# available by default.
# Usage: [PROFILE=headless] [DRY_RUN=1] ./plugins/install.sh
set -u
PROFILE="${PROFILE:-headless}"
DRY_RUN="${DRY_RUN:-0}"
PKGS="
@deepseek-ai/dsh-tool-present
@deepseek-ai/dsh-tool-ask-user
@deepseek-ai/dsh-tool-str-replace-editor
@deepseek-ai/dsh-skill-office
@deepseek-ai/dsh-hooks-claude-code
@deepseek-ai/dsh-tool-bash-persistent
@deepseek-ai/dsh-mcp-client
"
ok=0
fail=0
for p in $PKGS; do
  if [ "$DRY_RUN" = "1" ]; then
    echo "dsh plugin --profile $PROFILE add $p"
    ok=$((ok + 1))
  else
    if dsh plugin --profile "$PROFILE" add "$p"; then
      echo "installed: $p"
      ok=$((ok + 1))
    else
      echo "FAILED: $p" >&2
      fail=$((fail + 1))
    fi
  fi
done
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
for p in rdsh-settings; do
  if [ "$DRY_RUN" = "1" ]; then
    echo "dsh plugin --profile $PROFILE add $REPO_ROOT/plugins/$p"
    ok=$((ok + 1))
  else
    if dsh plugin --profile "$PROFILE" add "$REPO_ROOT/plugins/$p"; then
      echo "installed: $p (bundled)"
      ok=$((ok + 1))
    else
      echo "FAILED: $p" >&2
      fail=$((fail + 1))
    fi
  fi
done
echo "done: $ok ok, $fail failed (profile: $PROFILE)"
[ "$fail" -eq 0 ]
