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
notify() {
  log "notify: $1 -- $2"
  if command -v notify-send >/dev/null 2>&1; then
    notify-send "rdsh sync: $1" "$2" 2>/dev/null || true
  fi
}
if command -v flock >/dev/null 2>&1; then
  exec 9>"$LOCK" || exit 1
  flock -n 9 || { log "another sync is running; exit"; exit 0; }
fi
export PATH="$HOME/.local/bin:$HOME/.local/opt/node-v24.16.0-linux-x64/bin:$HOME/.cargo/bin:$PATH"
write_state() {
  printf "{\"updated\":true,\"kind\":\"%s\",\"from\":\"%s\",\"to\":\"%s\",\"at\":%s}\n" \
    "$1" "$2" "$3" "$(date +%s)000" > "$LOGDIR/update-state.json"
}
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
ALL="$("$NPM" view @deepseek-ai/dsh versions --json 2>/dev/null | tr -d " [],\"" | tr "," "\n" | grep -E "^[0-9]+\.[0-9]+\.[0-9]+" || true)"
if [ -z "$ALL" ]; then log "registry unreachable; try later"; exit 0; fi
pick() {
  case "$CHANNEL" in
    any) printf "%s\n" $ALL | sort -V | tail -n 1 ;;
    *) printf "%s\n" $ALL | grep -E "^[0-9]+\.[0-9]+\.[0-9]+(-rc\.[0-9]+)?$" | sort -V | tail -n 1 ;;
  esac
}
LATEST="$(pick)"
log "installed=$INSTALLED latest($CHANNEL)=$LATEST"
DSH_CHANGED=0
if [ "$INSTALLED" = "$LATEST" ]; then
  log "dsh up to date"
else
  DSH_CHANGED=1
  if [ "$CHECK_ONLY" = 1 ]; then log "dsh update available: $LATEST"; fi
fi
if [ "$DSH_CHANGED" = 1 ] && [ "$CHECK_ONLY" = 0 ]; then
log "updating $INSTALLED -> $LATEST"
if ! "$NPM" install -g "@deepseek-ai/dsh@$LATEST" >> "$LOGDIR/sync.log" 2>&1; then
  log "npm install failed; kept $INSTALLED"
  notify "dsh update failed" "npm install of $LATEST failed; kept $INSTALLED"
  exit 1
fi
GOT="$(node -p "require(process.argv[1]).version" "$PKGROOT/package.json" 2>/dev/null)"
ORIG_BIN="$(command -v dsh-orig 2>/dev/null || printf "%s" "$HOME/.local/bin/dsh-orig")"
if [ "$GOT" = "$LATEST" ] && [ -x "$ORIG_BIN" ] && "$ORIG_BIN" --version >/dev/null 2>&1; then
  log "updated OK: $GOT (orig binary answers)"
  write_state "dsh" "$INSTALLED" "$GOT"
  notify "dsh updated" "$INSTALLED -> $GOT"
else
  log "verify failed (got=$GOT); rolling back to $INSTALLED"
  notify "dsh update failed" "verify failed for $LATEST; rolled back to $INSTALLED"
  "$NPM" install -g "@deepseek-ai/dsh@$INSTALLED" >> "$LOGDIR/sync.log" 2>&1 || true
  log "rollback done"
  exit 1
fi
fi
REPO="$(cd "$(dirname "$0")" && pwd)"
if [ -x "$REPO/target/release/rdsh" ] && [ -f "$REPO/tests/regress.sh" ]; then
  if BIN="$REPO/target/release/rdsh" sh "$REPO/tests/regress.sh" >> "$LOGDIR/sync.log" 2>&1; then
    log "post-update regress: ALL PASS"
  else
    log "post-update regress: FAILURES (see above); dsh itself is updated, rdsh compat needs a look"
  fi
fi
if [ -d "$REPO/.git" ] && command -v git >/dev/null 2>&1; then
  if ! git -C "$REPO" fetch origin main >> "$LOGDIR/sync.log" 2>&1; then
    log "rdsh fetch failed; try later"
  else
    LOCAL="$(git -C "$REPO" rev-parse HEAD 2>/dev/null)"
    REMOTE="$(git -C "$REPO" rev-parse origin/main 2>/dev/null)"
    if [ -z "$LOCAL" ] || [ -z "$REMOTE" ]; then
      log "rdsh ref lookup failed"
    elif [ "$LOCAL" = "$REMOTE" ]; then
      log "rdsh up to date ($LOCAL)"
    elif [ "$CHECK_ONLY" = 1 ]; then
      log "rdsh update available: $REMOTE"
    elif [ -n "$(git -C "$REPO" status --porcelain 2>/dev/null)" ]; then
      log "rdsh tree dirty; skipping auto-update"
    elif ! git -C "$REPO" merge --ff-only "origin/main" >> "$LOGDIR/sync.log" 2>&1; then
      log "rdsh fast-forward failed; skipping"
    elif ! command -v cargo >/dev/null 2>&1; then
      log "cargo not found; pulled but not rebuilt"
    elif (cd "$REPO" && cargo build --release >> "$LOGDIR/sync.log" 2>&1) \
      && BIN="$REPO/target/release/rdsh" sh "$REPO/tests/regress.sh" >> "$LOGDIR/sync.log" 2>&1; then
      PREFIX_BIN="${PREFIX_BIN:-$HOME/.local/bin}"
      install -m755 "$REPO/target/release/rdsh" "$PREFIX_BIN/rdsh"
      if "$PREFIX_BIN/dsh" doctor 2>&1 | grep -q "rdsh"; then
        TMP="$PREFIX_BIN/.dsh.new.$$"
        install -m755 "$PREFIX_BIN/rdsh" "$TMP" && mv -f "$TMP" "$PREFIX_BIN/dsh"
      fi
      write_state "rdsh" "$LOCAL" "$REMOTE"
      log "rdsh updated OK: $REMOTE (regress passed, binaries refreshed)"
      notify "rdsh updated" "rebuilt and refreshed from $REMOTE"
    else
      log "rdsh build/regress failed; binaries untouched"
      notify "rdsh update failed" "build or regress failed; binaries untouched"
    fi
  fi
else
  log "rdsh repo unavailable; skipping"
fi
log "done"
