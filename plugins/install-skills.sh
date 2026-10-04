#!/bin/sh
# Install/refresh filesystem skills into $DSH_HOME/skills.
# Usage: [DSH_HOME=~/.dsh] [FORCE=1] [DRY_RUN=1] ./plugins/install-skills.sh
set -u
DSH_HOME="${DSH_HOME:-$HOME/.dsh}"
DRY_RUN="${DRY_RUN:-0}"
FORCE="${FORCE:-0}"
REF="${REF:-main}"
SKILLS="$DSH_HOME/skills"
TMP=/tmp/rdsh-skills-$$
ok=0
skip=0

need() { command -v "$1" >/dev/null 2>&1 || { echo "missing: $1" >&2; exit 1; }; }
need git

put_dir() {
  src="$1"; name="$2"
  dst="$SKILLS/$name"
  if [ "$DRY_RUN" = "1" ]; then
    if [ -d "$dst" ] && [ "$FORCE" != "1" ]; then
      echo "keep: $name (FORCE=1 to refresh)"
      skip=$((skip + 1))
    else
      echo "install: $name"
      ok=$((ok + 1))
    fi
    return
  fi
  if [ -d "$dst" ] && [ "$FORCE" != "1" ]; then
    echo "keep: $name (FORCE=1 to refresh)"
    skip=$((skip + 1))
    return
  fi
  if [ -d "$dst" ]; then
    mv "$dst" "$dst.bak-$$"
    echo "backup: $name -> $name.bak-$$"
  fi
  cp -r "$src" "$dst"
  echo "installed: $name"
  ok=$((ok + 1))
}

mkdir -p "$SKILLS"
if [ "$DRY_RUN" = "1" ]; then
  for n in ponytail ponytail-audit ponytail-debt ponytail-gain ponytail-help ponytail-review; do
    put_dir x "$n"
  done
else
  rm -rf "$TMP"
  git clone --depth 1 --branch "$REF" https://github.com/DietrichGebert/ponytail "$TMP" >&2
  for d in "$TMP"/.openclaw/skills/ponytail*; do
    [ -d "$d" ] || continue
    put_dir "$d" "$(basename "$d")"
  done
  rm -rf "$TMP"
fi

if command -v rtk >/dev/null 2>&1; then
  echo "rtk binary: $(rtk --version 2>/dev/null)"
else
  echo "rtk binary: MISSING (install from https://github.com/rtk-ai/rtk)" >&2
fi
if [ -f "$SKILLS/rtk/SKILL.md" ]; then
  echo "rtk skill: present"
else
  echo "rtk skill: MISSING in $SKILLS (kept as-is; place SKILL.md manually)" >&2
fi

echo "done: $ok installed, $skip kept (skills: $SKILLS)"
