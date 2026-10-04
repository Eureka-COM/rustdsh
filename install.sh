#!/usr/bin/env bash
# rdsh installer: install as `rdsh`, optionally shadow `dsh` (with backup + restore).
set -euo pipefail
PREFIX="${PREFIX:-$HOME/.local/bin}"
MODE="rdsh"
for a in "$@"; do
  case "$a" in
    --as-dsh) MODE="as-dsh" ;;
    --restore) MODE="restore" ;;
    --prefix=*) PREFIX="${a#--prefix=}" ;;
    -h|--help)
      echo "usage: ./install.sh [--as-dsh] [--restore] [--prefix=DIR]"
      echo "  (default)  build + install rdsh to PREFIX/rdsh"
      echo "  --as-dsh   also install rdsh as PREFIX/dsh (backs up original to PREFIX/dsh-orig)"
      echo "  --restore  restore PREFIX/dsh from PREFIX/dsh-orig"
      exit 0 ;;
    *) echo "unknown arg: $a" >&2; exit 2 ;;
  esac
done
if [ "$MODE" = restore ]; then
  if [ ! -e "$PREFIX/dsh-orig" ]; then echo "no backup at $PREFIX/dsh-orig" >&2; exit 1; fi
  if "$PREFIX/dsh" doctor 2>&1 | grep -q 'rdsh'; then
    mv -f "$PREFIX/dsh-orig" "$PREFIX/dsh"
    echo "restored original dsh (rdsh still at $PREFIX/rdsh)"
  else
    echo "refusing: $PREFIX/dsh does not look like rdsh" >&2; exit 1
  fi
  exit 0
fi
export PATH="$HOME/.cargo/bin:$PATH"
cargo build --release
mkdir -p "$PREFIX" "$HOME/.config/rdsh"
install -m755 target/release/rdsh "$PREFIX/rdsh"
echo "installed $PREFIX/rdsh"
if [ "$MODE" = as-dsh ]; then
  if [ -e "$PREFIX/dsh-orig" ]; then echo "backup exists: $PREFIX/dsh-orig (use --restore first)" >&2; exit 1; fi
  if [ -e "$PREFIX/dsh" ]; then
    if "$PREFIX/dsh" doctor 2>&1 | grep -q 'rdsh'; then
      echo "PREFIX/dsh is already rdsh; refreshing"
    else
      mv "$PREFIX/dsh" "$PREFIX/dsh-orig"
      echo "$PREFIX/dsh-orig" > "$HOME/.config/rdsh/origin"
      echo "backed up original dsh -> $PREFIX/dsh-orig"
    fi
  else
    echo "no existing dsh in PREFIX; PATH lookup only"
  fi
  tmp="$PREFIX/.dsh.new.$$"
  install -m755 "$PREFIX/rdsh" "$tmp" && mv -f "$tmp" "$PREFIX/dsh"
  echo "installed rdsh as $PREFIX/dsh (atomic replace; revert: ./install.sh --restore)"
fi
