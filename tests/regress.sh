#!/bin/sh
# rdsh CLI regression. Fails on first mismatch (set -e + explicit checks).
BIN="${BIN:-./target/release/rdsh}"
pass=0
ok() { pass=$((pass+1)); echo "ok: $1"; }
need_ok() {
  desc="$1"; shift
  if "$@" >/tmp/rr-out 2>/tmp/rr-err; then ok "$desc"; else echo "FAIL(exit): $desc"; cat /tmp/rr-err; exit 1; fi
}
need_exit() {
  want="$1"; desc="$2"; shift 2
  "$@" >/tmp/rr-out 2>/tmp/rr-err
  code=$?
  if [ "$code" = "$want" ]; then ok "$desc"; else echo "FAIL(exit $code want $want): $desc"; exit 1; fi
}
need_grep() {
  pat="$1"; desc="$2"; shift 2
  "$@" >/tmp/rr-out 2>/tmp/rr-err
  if grep -q "$pat" /tmp/rr-out; then ok "$desc"; else echo "FAIL(output): $desc"; exit 1; fi
}
need_grep "rdsh" "version string" $BIN --version
need_ok "doctor" $BIN doctor
printf "hello world, this is a token test" | $BIN tokens > /tmp/rr-out 2>/dev/null
if grep -q "\"tokens\": 9" /tmp/rr-out; then ok "tokens stdin"; else echo "FAIL(output): tokens stdin"; exit 1; fi
need_ok "search" $BIN search estimate_tokens --dir src --max 5
need_ok "profiles" $BIN profiles
need_ok "skills" $BIN skills
need_ok "logs" $BIN logs --tail 3
need_ok "sessions" $BIN sessions --limit 2
need_ok "dump-native" $BIN dump-config --profile tui --native
need_ok "boot dry-run" $BIN --dry-run tui
printf "ls /tmp" | $BIN guard --deny "zzz-no-match" > /dev/null
if [ $? -eq 0 ]; then ok "guard allow"; else echo "FAIL: guard allow"; exit 1; fi
printf "run this" | $BIN guard --deny "run*" > /dev/null 2>&1
if [ $? -eq 2 ]; then ok "guard block"; else echo "FAIL: guard block"; exit 1; fi
need_exit 2 "reject desktop" $BIN desktop
need_exit 2 "reject double profile" $BIN --profile a --profile b
need_exit 2 "reject dump with args" $BIN --profile tui --dump-config --foo
need_exit 2 "reject plugin w/o args" $BIN plugin --profile tui
python3 -c "print(5791 * 4)" | $BIN tokens > /dev/null
printf "FROMSTDIN" > /tmp/rr-in.txt
need_ok "compact noop" $BIN compact /tmp/rr-in.txt --max-tokens 8000
SB=/tmp/rdsh-regress-$$
mkdir -p $SB/bin $SB/orig
cat > $SB/orig/dsh << FAKEEOF
#!/bin/sh
echo FAKE-ORIG
FAKEEOF
chmod +x $SB/orig/dsh
cp "$BIN" $SB/bin/dsh
if PATH="$SB/bin:$SB/orig:$PATH" DSH_ORIG_BIN="$SB/orig/dsh" $SB/bin/dsh --version | grep -q FAKE-ORIG; then ok "dsh-mode delegates"; else echo "FAIL: dsh-mode delegates"; exit 1; fi
if PATH="$SB/bin:$SB/orig:$PATH" DSH_ORIG_BIN="$SB/orig/dsh" $SB/bin/dsh doctor | grep -q rdsh; then ok "dsh-mode native"; else echo "FAIL: dsh-mode native"; exit 1; fi
rm -rf $SB /tmp/rr-in.txt /tmp/rr-out /tmp/rr-err
echo "ALL PASS ($pass checks)"
