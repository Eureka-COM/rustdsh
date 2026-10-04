#!/bin/sh
# Install rdsh-update-banner into a dsh web profile (idempotent).
# Usage: install-banner.sh [--profile=web] [--from-dir=DIR] [--from-npm] [--demo]
#   default: link the copy bundled in this repo (plugins/rdsh-update-banner)
#   --from-dir: link a different local checkout instead (dev use)
#   --from-npm: install from the npm registry instead of linking
set -u
PROFILE="web"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
FROM_DIR="$SCRIPT_DIR/plugins/rdsh-update-banner"
FROM_NPM=0
DEMO="false"
for a in "$@"; do
  case "$a" in
    --profile=*) PROFILE="${a#--profile=}" ;;
    --from-dir=*) FROM_DIR="${a#--from-dir=}" ;;
    --from-npm) FROM_NPM=1 ;;
    --demo) DEMO="true" ;;
    -h|--help) echo "usage: install-banner.sh [--profile=web] [--from-dir=DIR] [--from-npm] [--demo]"; exit 0 ;;
    *) echo "unknown arg: $a" >&2; exit 2 ;;
  esac
done
DSH_HOME_DIR="${DSH_HOME:-$HOME/.dsh}"
PDIR="$DSH_HOME_DIR/profiles/$PROFILE"
if [ ! -d "$PDIR" ]; then echo "no such profile dir: $PDIR" >&2; exit 1; fi
if [ "$FROM_NPM" = 1 ]; then
  if ! dsh plugin --profile "$PROFILE" add rdsh-update-banner; then
    echo "plugin install failed" >&2; exit 1
  fi
else
  if [ ! -f "$FROM_DIR/package.json" ]; then echo "no package.json in $FROM_DIR" >&2; exit 1; fi
  mkdir -p "$PDIR/node_modules"
  ln -sfn "$FROM_DIR" "$PDIR/node_modules/rdsh-update-banner"
  echo "linked $FROM_DIR -> $PDIR/node_modules/rdsh-update-banner"
fi
PATCH="$PDIR/cordis.patch.yml"
if [ ! -f "$PATCH" ]; then echo "no patch file: $PATCH" >&2; exit 1; fi
if grep -q "id: rdsh-update-banner" "$PATCH"; then
  echo "already wired in $PATCH"
  exit 0
fi
cp "$PATCH" "$PATCH.bak"
cat >> "$PATCH" << YMLEOF
- insert:
    - id: rdsh-update-banner
      name: "rdsh-update-banner"
      config:
        demo: $DEMO
YMLEOF
echo "appended insert block to $PATCH (backup: $PATCH.bak)"
echo "reload the web GUI to pick it up"
