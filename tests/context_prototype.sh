#!/bin/sh
# context engine prototype regression (sandboxed, never touches real ~/.dsh).
BIN="${BIN:-./target/debug/rdsh}"
pass=0
ok() { pass=$((pass+1)); echo "ok: $1"; }
if [ "${RDSH_CTX_SANDBOXED:-}" != 1 ]; then
  SB="$(mktemp -d 2>/dev/null || mktemp -d -t rdsh-ctx)"
  mkdir -p "$SB/home" "$SB/dsh" "$SB/work"
  HOME="$SB/home"; DSH_HOME="$SB/dsh"; RDSH_CTX_SANDBOXED=1
  export HOME DSH_HOME RDSH_CTX_SANDBOXED SB
  trap 'rm -rf "$SB"' EXIT INT TERM
fi
need_ok() {
  desc="$1"; shift
  if "$@" >/tmp/ctx-out 2>/tmp/ctx-err; then ok "$desc"; else echo "FAIL(exit): $desc"; cat /tmp/ctx-err; exit 1; fi
}
need_grep() {
  pat="$1"; desc="$2"; shift 2
  "$@" >/tmp/ctx-out 2>/tmp/ctx-err
  if grep -q "$pat" /tmp/ctx-out; then ok "$desc"; else echo "FAIL(output): $desc (want $pat)"; cat /tmp/ctx-out; exit 1; fi
}
need_absent() {
  pat="$1"; desc="$2"; shift 2
  "$@" >/tmp/ctx-out 2>/tmp/ctx-err
  rc=$?
  if [ $rc -ne 0 ]; then echo "FAIL(exit): $desc"; cat /tmp/ctx-err; exit 1; fi
  if grep -q "$pat" /tmp/ctx-out; then echo "FAIL(present): $desc (must not contain $pat)"; cat /tmp/ctx-out; exit 1; else ok "$desc"; fi
}
need_ok "context status default" $BIN context status
need_grep "prototype" "status json flag" $BIN context status --json
printf "%s" "{\"goal\":\"dsh互換性を維持する\",\"working_files\":[\"src/search.rs\"],\"open_tasks\":[\"GDN向けpacking評価\"],\"decisions\":[\"agent loopは再実装しない\"],\"token_budget\":4000}" > "$DSH_HOME/rdsh-context.json"
need_grep "dsh" "build uses goal" $BIN context build --json
need_grep "goal" "explain lists priority" $BIN context explain
need_ok "search runs" $BIN context search tokens --max 5
rm -f "$DSH_HOME/rdsh-context.json"
printf '%s' '{"schema":1,"context":{"goal":"fallback-goal-xyz","max_code_hits":5,"max_sessions":3,"include_git_diff":false}}' > "$DSH_HOME/rdsh.json"
need_grep "fallback-goal-xyz" "fallback reads rdsh.json context" $BIN context status --json
need_grep '"max_code_hits": 5' "fallback applies knobs" $BIN context status --json
: > "$DSH_HOME/rdsh-context.json"
need_grep "fallback-goal-xyz" "fallback on empty context file" $BIN context status --json
printf '%s' '{"goal":"ctx-file-goal-xyz","token_budget":4000}' > "$DSH_HOME/rdsh-context.json"
need_grep "ctx-file-goal-xyz" "context file keeps priority" $BIN context status --json
printf '%s' '{"goal":"g","token_budget":999999999,"max_code_hits":9999,"max_sessions":9999}' > "$DSH_HOME/rdsh-context.json"
need_grep '"token_budget": 200000' "clamp token_budget" $BIN context status --json
need_grep '"max_code_hits": 100' "clamp max_code_hits" $BIN context status --json
need_grep '"max_sessions": 20' "clamp max_sessions" $BIN context status --json
printf '%s' '{"goal":"g","max_sessions":0}' > "$DSH_HOME/rdsh-context.json"
need_grep '"max_sessions": 0' "max_sessions allows zero" $BIN context status --json
printf '%s' '{"goal":"g","include_git_diff":false}' > "$DSH_HOME/rdsh-context.json"
need_absent '"name": "git_diff"' "git_diff omitted when OFF" $BIN context build --json
need_grep "git_diff=off" "status shows git_diff off" $BIN context status
printf '%s' '{"goal":"g"}' > "$DSH_HOME/rdsh-context.json"
need_grep '"name": "git_diff"' "git_diff present by default" $BIN context build --json
echo "context-prototype: ALL PASS ($pass)"
