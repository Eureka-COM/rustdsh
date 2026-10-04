# rdsh — dsh の Rust 高速ランチャー（安全な移植）

[English](README.md)

フル移植ではなく**ホットパスだけ Rust 化＋残りは本家 dsh に委譲**する設計です。
起動約81倍・メモリ約1/20を、本家の動作を変えずに実現します。

- 起動中央値 **約1.19ミリ秒**（本家約96.7ミリ秒）
- 常駐メモリ **約2.9MB**（本家約66MB）・単一バイナリ約799KB（依存ツリー不要）
- `dsh`名で置換しても引数を一字も変えず委譲するため、既存の使い方・スクリプトはそのまま動きます

## 目次

- [実測](#実測)
- [インストール](#インストール)
- [使い方](#使い方)
- [置換モード（dsh として使う）](#置換モードdsh-として使う)
- [Web UI（ダッシュボード）](#web-uiダッシュボード)
- [安全設計](#安全設計)
- [高速化の仕組み](#高速化の仕組み)
- [構成](#構成)
- [よくある質問](#よくある質問)
- [ライセンス](#ライセンス)

## 実測

手元環境（Linux x86_64）での測定値です。条件をそろえた前後比較も含みます。

| 項目 | rdsh | 比較対象 | 倍率 |
|---|---|---|---|
| `--version` 起動（中央値、n=5） | 約1.19ms | 本家dsh 約96.7ms | 約81倍 |
| `--version` メモリ（最大RSS） | 約2.9MB | 本家 約66MB | 約1/23 |
| フック相当処理のメモリ | 約2.7MB | node同等 約45MB | 約1/16 |
| search（300ファイル・60万行） | 約12ms | 改修前 約41ms | 約3.4倍 |
| tokens（9.6MBテキスト） | 約12ms | 改修前 約35ms | 約2.9倍 |
| sessions --tokens（20件展開） | 約0.41秒 | 改修前 約1.65秒 | 約4.0倍 |
| 配布サイズ | 単一バイナリ約799KB | Nodeツリー約508MB | — |

測定コマンドは `rdsh bench --n 5` と `/usr/bin/time -v` です。再現手順は[高速化の仕組み](#高速化の仕組み)にあります。

## インストール

```sh
git clone https://github.com/sahenjp/rustdsh.git
cd rustdsh
./install.sh                 # ビルド＋ ~/.local/bin/rdsh に導入
./install.sh --as-dsh        # rdsh を `dsh` 名でも使えるよう置換（元は dsh-orig に退避）
./install.sh --restore       # 置換を元に戻す
./install.sh --prefix=DIR    # 導入先を変更（既定 ~/.local/bin）
```

ソースから直接建てる場合は `cargo build --release` で `target/release/rdsh` ができます。

## 使い方

### dsh 互換（委譲）

```sh
rdsh tui                          # = dsh --profile tui（slim env 付きで委譲）
rdsh --profile web --patch x.yml  # オーバーレイ付き起動
rdsh --passthrough tui            # slim 無しの完全委譲（非常口）
rdsh --dry-run tui -- --resume abc  # 実行内容だけ表示
```

### 高速ネイティブコマンド（Nodeを起動しない）

```sh
rdsh tokens ./AGENTS.md             # 入力トークン見積（約4文字=1トークン、CJKは1字1トークン）
echo ... | rdsh prune --max-tokens 4000   # head+tailを残して予算内に切り詰め
rdsh search TODO --dir . --max 100 # 再帰grep（並列・出力順は逐次と同一）
rdsh compact ./s.jsonl --max-tokens 8000 # セッションJSONLの圧縮（元ファイル不変）
rdsh sessions --limit 20 --tokens  # セッション一覧＋展開後トークン見積
rdsh logs --tail 50 --grep ERROR   # 起動ログの参照
rdsh profiles / rdsh skills        # プロファイル・スキル一覧
rdsh doctor                        # 本家dsh・DSH_HOME・slim設定の確認
rdsh bench --n 5                   # rdsh/dsh の起動比較
rdsh serve                         # Webダッシュボード（:3080）
```

### hooks.json での使い方（`rdsh guard`）

標準入力（フックJSONまたは生テキスト）を走査し、拒否パターンに一致したらexit 2＋理由出力でブロック、それ以外はexit 0で通過します。`--json` で `{"decision":"block"/"approve"}` を返します。パターンの `*` は任意文字列に一致します。

```sh
echo "$input" | rdsh guard --deny "rm -rf /*" --deny "*token*"
```

```json
{
  "hooks": {
    "PreToolUse": [
      { "matcher": "Bash", "hooks": [{ "type": "command", "command": "rdsh guard --deny \"rm -rf /*\"" }] }
    ]
  }
}
```

## 置換モード（dsh として使う）

`dsh`名で呼ばれた場合の振る舞いです。

- rdsh固有の先頭サブコマンド（`tokens`/`guard`/`serve`/`sessions`等）以外は、**引数を一字も変えず本家へexec委譲**します（`dsh --version`・`dsh --profile tui`・`dsh --help`は完全互換）
- 本家の探索順： `DSH_ORIG_BIN` → `~/.config/rdsh/origin` → 退避ファイル（dsh-orig等）→ PATH（自分を除外）→ 既知npmパス
- 一時退避： `RDSH_PASSTHROUGH=1 dsh ...`（slim無し）、`RDSH_DRY_RUN=1 dsh ...`（実行内容のみ表示）
- 注意： `dsh tokens` のようにプロファイル名が予約語と衝突する場合は `dsh --profile tokens` で起動してください

## Web UI（ダッシュボード）

```sh
rdsh serve
# → http://127.0.0.1:3080/ を開く（localhost のみ、読取専用API）
# ※ dsh web GUIと同ポートのため競合時は `rdsh serve --port 38080` 等を使ってください
```

| API | 内容 |
|---|---|
| `GET /api/version` | バージョン |
| `GET /api/doctor` | 状態確認 |
| `POST /api/tokens` | トークン推定（`{"text"}`） |
| `POST /api/prune` | 切り詰め（`{"text","max_tokens"}`） |
| `GET /api/bench?n=3` | 起動計測 |
| `GET /api/sessions?limit=20` | セッション一覧 |
| `GET /api/skills` / `/api/profiles` | 一覧 |

依存なし（std のみ＋埋め込み単一HTML、CDN不要・オフライン可）です。

## 安全設計

1. agent loop・profile bootの再実装はしません。`exec`委譲のみです
2. slimは**環境変数の追加だけ**です。本家が知らないキーは無視されます
3. `desktop`プロファイル拒否・dump排他など本家のエラー条件をRust側でも再現します
4. 読取系（tokens/search/compact/dump --native/serve API/inspect）は元ファイルを書き換えません
5. `--passthrough`・`RDSH_PASSTHROUGH=1`・`./install.sh --restore`で即時退避できます

### 検証（すべて実行済み）

- `cargo test`：14件通過（トークン計算・ワイルドカード・引数分割）
- `tests/regress.sh`：19件通過（全サブコマンド・異常系・dsh名委譲の隔離検証）
- 高速化の前後で出力をdiff比較し、完全一致を確認（300件search・上限打ち切りsearch）
- 実置換後に `dsh --version`（委譲）と `dsh guard`（新機能）を実機確認

## 高速化の仕組み

- トークン推定のASCII高速路：純ASCIIは `len/4` 一発計算（非ASCIIのみ従来走査、結果は同一）
- searchの二段階化：逐次walkで順序固定→ファイル単位で並列grep→walk順に結合。32ファイル未満は従来の逐次路のままです
- sessions --tokensの展開並列化：zstd展開をスレッド分散（数値は逐次と同一、順序保持）
- ビルドは `opt-level=z`＋LTO＋strip＋`panic=abort` で小型維持（約799KB）
- 再現： `python3` で9.6MBテキスト・300ファイル合成木を作り、新旧バイナリを `time` 比較（旧版はgit worktreeでHEADビルド）

## 構成

- `src/main.rs` — CLI定義・振り分け・`dsh`名検出
- `src/dsh_args.rs` — 本家 `lib/bin.js` 互換の引数分割（読取専用）
- `src/passthrough.rs` — 本家探索＋`exec`委譲
- `src/slim.rs` — slim env定義
- `src/tokens.rs` — トークン推定・prune
- `src/search.rs` — 順序保持の並列grep
- `src/compact.rs` — JSONLセッション圧縮
- `src/inspect.rs` — sessions/logs/skills/profiles参照
- `src/guard.rs` — hooks.json用ガード
- `src/serve.rs`＋`src/ui.html` — ローカルWeb UI
- `install.sh` — 導入（`--as-dsh`置換／`--restore`復元）
- `tests/regress.sh` — CLI回帰試験（19件）

## よくある質問

- **3080が使用中と言われる**：dsh web GUIと同ポートです。`rdsh serve --port 38080` を使ってください
- **プロファイル名がサブコマンドと被る**：`dsh --profile <name>` 形式で起動してください
- **元に戻したい**：`./install.sh --restore`（退避した本家を復元）
- **`--tokens` の `?` 付き表示**：zstd CLIが無い環境では圧縮サイズからの概算である印です

## ライセンス

MIT（[LICENSE](LICENSE)）です。
