#!/bin/sh
# Keep the original dsh in step with upstream releases (rdsh itself needs no
# rebuild: it delegates by exec). Safe by default: verify after update,
# roll back to the previous version on any failure.
# Usage: sync-dsh.sh [--check-only] [--channel rc|any]  (default channel: rc)
set -u
CHANNEL="rc"
CHECK_ONLY=0
for a in "$@"; do
  case "$a" in
    --check-only) CHECK_ONLY=1 ;;
    --channel=*) CHANNEL="${a#--channel=}" ;;
    -h|--help) echo "usage: sync-dsh.sh [--check-only] [--channel rc|any]"; exit 0 ;;
    *) echo "unknown arg: $a" >&2; exit 2 ;;
  esac
done
LOGDIR="${HOME}/.local/share/rdsh"
LOCK="/tmp/rdsh-sync.lock"
mkdir -p "$LOGDIR"
log() { printf "%s %s\n" "$(date -u +%FT%TZ)" "$*" | tee -a "$LOGDIR/sync.log"; }
if command -v flock >/dev/null 2>&1; then
  exec 9>"$LOCK" || exit 1
  flock -n 9 || { log "another sync is running; exit"; exit 0; }
fi
export PATH="$HOME/.local/bin:$HOME/.local/opt/node-v24.16.0-linux-x64/bin:$PATH"
NPM=""
for cand in "${NPM_BIN:-}" "$HOME/.local/bin/npm" "$HOME/.local/opt/node-v24.16.0-linux-x64/bin/npm" "$(command -v npm 2>/dev/null)"; do
  if [ -n "$cand" ] && [ -x "$cand" ]; then NPM="$cand"; break; fi
done
if [ -z "$NPM" ]; then log "npm not found; set NPM_BIN"; exit 1; fi
log "using npm: $NPM"
PKGROOT="$("$NPM" root -g 2>/dev/null)/@deepseek-ai/dsh"
if [ ! -f "$PKGROOT/package.json" ]; then log "dsh package not found under $PKGROOT"; exit 1; fi
INSTALLED="$(node -p "require(process.argv[1]).version" "$PKGROOT/package.json" 2>/dev/null)"
if [ -z "$INSTALLED" ]; then log "cannot read installed version"; exit 1; fi
ALL="$(npm view @deepseek-ai/dsh versions --json 2>/dev/null | tr -d " [],\"" | tr "," "\n" | grep -E "^[0-9]+\.[0-9]+\.[0-9]+" || true)"
if [ -z "$ALL" ]; then log "registry unreachable; try later"; exit 0; fi
pick() {
  case "$CHANNEL" in
    any) printf "%s\n" $ALL | sort -V | tail -n 1 ;;
    *) printf "%s\n" $ALL | grep -E "^[0-9]+\.[0-9]+\.[0-9]+(-rc\.[0-9]+)?$" | sort -V | tail -n 1 ;;
  esac
}
LATEST="$(pick)"
log "installed=$INSTALLED latest($CHANNEL)=$LATEST"
if [ "$INSTALLED" = "$LATEST" ]; then log "up to date"; exit 0; fi
if [ "$CHECK_ONLY" = 1 ]; then log "update available: $LATEST"; exit 0; fi
log "updating $INSTALLED -> $LATEST"
if ! "$NPM" install -g "@deepseek-ai/dsh@$LATEST" >> "$LOGDIR/sync.log" 2>&1; then
  log "npm install failed; kept $INSTALLED"; exit 1
fi
GOT="$(node -p "require(process.argv[1]).version" "$PKGROOT/package.json" 2>/dev/null)"
ORIG_BIN="$(command -v dsh-orig 2>/dev/null || printf "%s" "$HOME/.local/bin/dsh-orig")"
if [ "$GOT" = "$LATEST" ] && [ -x "$ORIG_BIN" ] && "$ORIG_BIN" --version >/dev/null 2>&1; then
  log "updated OK: $GOT (orig binary answers)"
else
  log "verify failed (got=$GOT); rolling back to $INSTALLED"
  "$NPM" install -g "@deepseek-ai/dsh@$INSTALLED" >> "$LOGDIR/sync.log" 2>&1 || true
  log "rollback done"
  exit 1
fi
REPO="$(cd "$(dirname "$0")" && pwd)"
if [ -x "$REPO/target/release/rdsh" ] && [ -f "$REPO/tests/regress.sh" ]; then
  if BIN="$REPO/target/release/rdsh" sh "$REPO/tests/regress.sh" >> "$LOGDIR/sync.log" 2>&1; then
    log "post-update regress: ALL PASS"
  else
    log "post-update regress: FAILURES (see above); dsh itself is updated, rdsh compat needs a look"
  fi
fi
log "done"
