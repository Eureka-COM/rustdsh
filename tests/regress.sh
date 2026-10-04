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
need_grep "plugin" "plugin delegation dry-run" $BIN --dry-run plugin --profile web add ./dsh-notify-push
need_grep "smart-dsh" "doctor reports smart-dsh" $BIN doctor
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
if PATH="$SB/bin:$SB/orig:$PATH" RDSH_ORIG_BIN="$SB/orig/dsh" $SB/bin/dsh --version | grep -q FAKE-ORIG; then ok "RDSH_ORIG_BIN primary"; else echo "FAIL: RDSH_ORIG_BIN primary"; exit 1; fi
WB=/tmp/rdsh-wrapper-$$
mkdir -p $WB/.local/bin
printf '#!/bin/sh\nexec node "$(readlink -f "$(command -v dsh)")" --profile web\n' > $WB/.local/bin/dsh-web-local
if HOME="$WB" $BIN doctor 2>/dev/null | grep -q "dsh-web-local"; then ok "doctor flags node-on-dsh wrapper"; else echo "FAIL(output): doctor flags node-on-dsh wrapper"; exit 1; fi
rm -rf $WB
AB=/tmp/rdsh-auth-AA
mkdir -p $AB/home/.codex $AB/home/.local/share/opencode $AB/dsh
printf '%s' '{"tokens":{"access_token":"a","refresh_token":"r","account_id":"1"}}' > $AB/home/.codex/auth.json
printf '%s' '{"openai":{"type":"oauth","refresh":"r2","access":"a2","expires":1991708802841,"accountId":"9"}}' > $AB/home/.local/share/opencode/auth.json
if HOME="$AB/home" DSH_HOME="$AB/dsh" $BIN auth 2>/dev/null | grep -q "openai-codex"; then ok "auth detects opencode login"; else echo "FAIL(output): auth detects opencode login"; exit 1; fi
if HOME="$AB/home" DSH_HOME="$AB/dsh" $BIN auth --import >/dev/null 2>&1 && grep -q "llm-pi-ai/openai-codex" "$AB/dsh/.credentials.yaml"; then ok "auth import writes record"; else echo "FAIL(output): auth import writes record"; exit 1; fi
if HOME="$AB/home" DSH_HOME="$AB/dsh" $BIN auth 2>/dev/null | grep -q "already recognized"; then ok "auth import recognized"; else echo "FAIL(output): auth import recognized"; exit 1; fi
fmode="$(stat -c %a "$AB/dsh/.credentials.yaml" 2>/dev/null || stat -f "%Lp" "$AB/dsh/.credentials.yaml")"
if [ "$fmode" = "600" ]; then ok "auth file mode 600"; else echo "FAIL(mode): auth file mode"; exit 1; fi
if HOME="$AB/home" DSH_HOME="$AB/dsh" $BIN setup --json 2>/dev/null | grep -q "\"needed\":false"; then ok "setup connected"; else echo "FAIL(output): setup connected"; exit 1; fi
SB2=/tmp/rdsh-setup-AA
mkdir -p $SB2/home $SB2/dsh
if env -u DEEPSEEK_API_KEY -u OPENAI_API_KEY -u ANTHROPIC_API_KEY HOME="$SB2/home" DSH_HOME="$SB2/dsh" $BIN setup --json 2>/dev/null | grep -q "\"needed\":true"; then ok "setup needed on first run"; else echo "FAIL(output): setup needed on first run"; exit 1; fi
if env -u DEEPSEEK_API_KEY -u OPENAI_API_KEY -u ANTHROPIC_API_KEY HOME="$SB2/home" DSH_HOME="$SB2/dsh" DSH_ORIG_BIN="$SB/orig/dsh" $SB/bin/dsh --profile tui 2>&1 | grep -q "rdsh setup"; then ok "first-boot banner"; else echo "FAIL(output): first-boot banner"; exit 1; fi
rm -rf $SB2
FR=/tmp/rdsh-fr-AA
mkdir -p $FR/pkg $FR/bin $FR/latest/download
cp "$BIN" $FR/pkg/rdsh
for a in rdsh-linux-x64 rdsh-macos-arm64 rdsh-macos-x64; do tar -czf "$FR/latest/download/$a.tar.gz" -C $FR/pkg rdsh; done
if RDSH_RELEASE_BASE="file://$FR" DSH_HOME="$FR/dsh" sh ./install.sh --from-release --prefix="$FR/bin" >/dev/null 2>&1 && "$FR/bin/rdsh" --version 2>/dev/null | grep -q "rdsh"; then ok "from-release install"; else echo "FAIL(output): from-release install"; exit 1; fi
rm -rf $FR
rm -rf $AB
rm -rf $SB /tmp/rr-in.txt /tmp/rr-out /tmp/rr-err
echo "ALL PASS ($pass checks)"
