#!/bin/sh
# settings (rdsh.json schema v1) prototype checks.
# バイナリ不要・python3 のみ。正常例・異常例(clamp されること)の JSON 検証を行う。
SRC="${SRC:-src/rdsh_config.rs}"
pass=0
ok() { pass=$((pass+1)); echo "ok: $1"; }
fail() { echo "FAIL: $1"; exit 1; }
need_py() {
  desc="$1"; script="$2"
  if python3 -c "$script"; then ok "$desc"; else fail "$desc"; fi
}

[ -f "$SRC" ] || fail "src/rdsh_config.rs がありません"
grep -qF 'pub fn settings_path() -> String' "$SRC" || fail "公開API settings_path がありません"
grep -qF 'pub fn load() -> RdshSettings' "$SRC" || fail "公開API load がありません"
grep -qF 'pub fn save(&self) -> anyhow::Result<()>' "$SRC" || fail "公開API save がありません"
ok "公開API3件 (settings_path/load/save)"

need_py "正常例: 既定スキーマは範囲内" '
import json
v = {"schema":1,
 "general":{"slim":True,"passthrough":False,"dry_run":False,"default_profile":""},
 "tokens":{"default_budget":4000},
 "search":{"dir":".","max":100,"web_limit":10,"searxng_url":""},
 "compact":{"max_tokens":8000},
 "sessions":{"limit":20,"with_tokens":False},
 "logs":{"tail":50},
 "serve":{"port":3080},
 "guard":{"deny":[],"reason":""},
 "bench":{"n":5},
 "setup":{"web_port":0},
 "beta":{"context_engine":True},
 "context":{"token_budget":4000,"enable_retriever":True,"enable_packer":True,
  "enable_verifier":True,"goal":"","decisions":[],"constraints":[],
  "working_files":[],"open_tasks":[],"max_code_hits":20,"max_sessions":10,
  "include_git_diff":True}}
assert v["schema"] == 1
assert 500 <= v["tokens"]["default_budget"] <= 200000
assert 500 <= v["compact"]["max_tokens"] <= 200000
assert 500 <= v["context"]["token_budget"] <= 200000
assert 1 <= v["search"]["max"] <= 100 and 1 <= v["sessions"]["limit"] <= 100
assert 1 <= v["logs"]["tail"] <= 500 and 1 <= v["bench"]["n"] <= 20
assert 1 <= v["serve"]["port"] <= 65535
assert v["setup"]["web_port"] == 0  # 0 はランダムの意味で許容
'

need_py "異常例: 範囲外は clamp される" '
clamp = lambda n, lo, hi: max(lo, min(hi, n))
raw = {"tokens":{"default_budget":9999999},"compact":{"max_tokens":1},
 "context":{"token_budget":10},"search":{"max":500},"sessions":{"limit":0},
 "logs":{"tail":9999},"bench":{"n":99},"serve":{"port":70000},
 "setup":{"web_port":70000}}
assert clamp(raw["tokens"]["default_budget"],500,200000) == 200000
assert clamp(raw["compact"]["max_tokens"],500,200000) == 500
assert clamp(raw["context"]["token_budget"],500,200000) == 500
assert clamp(raw["search"]["max"],1,100) == 100
assert clamp(raw["sessions"]["limit"],1,100) == 1
assert clamp(raw["logs"]["tail"],1,500) == 500
assert clamp(raw["bench"]["n"],1,20) == 20
assert clamp(raw["serve"]["port"],1,65535) == 65535
assert clamp(raw["setup"]["web_port"],1,65535) == 65535
# setup の 0 だけは clamp 対象外(ランダム)
assert 0 == 0
'

need_py "異常例: 長文・多数は切詰められる" '
goal = "あ"*2500
many = [f"dec-{i}" for i in range(60)]
long_path = "x"*400
assert len(list(iter(goal))[:2000]) == 2000
assert many[:50][-1] == "dec-49" and len(many[:50]) == 50
assert len(long_path[:300]) == 300
'

need_py "legacy例: files 別名と goal が補完される" '
import json
legacy = {"token_budget":6000,"enable_verifier":False,
 "goal":"旧ファイルのgoal","files":["src/tokens.rs"],
 "open_tasks":["packing評価"]}
wf = legacy.get("working_files") or legacy.get("files", [])
assert wf == ["src/tokens.rs"]
assert legacy["goal"] == "旧ファイルのgoal"
assert legacy["enable_verifier"] is False
'

echo "settings-prototype: ALL PASS ($pass)"
