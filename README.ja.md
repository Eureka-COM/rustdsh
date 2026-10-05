# rdsh — dsh の Rust 高速ランチャー（安全な移植）

[English](README.md)

[![ci](https://github.com/sahenjp/rustdsh/actions/workflows/ci.yml/badge.svg)](https://github.com/sahenjp/rustdsh/actions/workflows/ci.yml)
[![dashboard](https://github.com/sahenjp/rustdsh/actions/workflows/dashboard.yml/badge.svg)](https://github.com/sahenjp/rustdsh/actions/workflows/dashboard.yml)
[![docs](https://github.com/sahenjp/rustdsh/actions/workflows/docs.yml/badge.svg)](https://github.com/sahenjp/rustdsh/actions/workflows/docs.yml)
[![release](https://img.shields.io/github/v/release/sahenjp/rustdsh.svg)](https://github.com/sahenjp/rustdsh/releases)
[![license](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

<img src="assets/icon.svg" width="96" alt="rdsh icon">

フル移植ではなく**ホットパスだけ Rust 化＋残りは本家 dsh に委譲**する設計です。
起動約98倍・メモリ約1/23を、本家の動作を変えずに実現します。

- 起動中央値 **約0.90ミリ秒**（本家約88ミリ秒）
- 常駐メモリ **約2.9MB**（本家約66MB）・単一バイナリ約806KB（依存ツリー不要）
- `dsh`名で置換しても引数を一字も変えず委譲するため、既存の使い方・スクリプトはそのまま動きます

## 目次

- [実測](#実測)
- [インストール](#インストール)
- [使い方](#使い方)
- [置換モード（dsh として使う）](#置換モードdsh-として使う)
- [Smart-DSH との併用](#smart-dsh-との併用)
- [Web UI（ダッシュボード）](#web-uiダッシュボード)
- [安全設計](#安全設計)
- [高速化の仕組み](#高速化の仕組み)
- [構成](#構成)
- [コミュニティ](#コミュニティ)
- [よくある質問](#よくある質問)
- [クレジット](#クレジット)
- [ライセンス](#ライセンス)

## 実測

手元環境（Linux x86_64）での測定値です。条件をそろえた前後比較も含みます。

| 項目 | rdsh | 比較対象 | 倍率 |
| --- | --- | --- | --- |
| `--version` 起動（中央値、n=5） | 約0.90ms | 本家dsh 約88ms | 約98倍 |
| `--version` メモリ（最大RSS） | 約2.9MB | 本家 約66MB | 約1/23 |
| フック相当処理のメモリ | 約2.7MB | node同等 約45MB | 約1/16 |
| search（300ファイル・60万行） | 約17ms | 改修前 約41ms | 約2.4倍 |
| tokens（9.6MBテキスト） | 約12ms | 改修前 約35ms | 約2.9倍 |
| sessions --tokens（20件展開） | 約0.41秒 | 改修前 約1.65秒 | 約4.0倍 |
| 配布サイズ | 単一バイナリ約806KB | Nodeツリー約508MB | — |

測定コマンドは `rdsh bench --n 5` と `/usr/bin/time -v` です。再現手順は[高速化の仕組み](#高速化の仕組み)にあります。

## インストール

いちばん速い方法（ビルド済みバイナリ、Rust不要）：

```sh
# Linux / macOS / WSL
curl -fsSL https://github.com/sahenjp/rustdsh/releases/latest/download/install.sh | bash -s -- --from-release
```

```powershell
# Windows（PowerShell）
& ([scriptblock]::Create((Invoke-WebRequest -Uri https://github.com/sahenjp/rustdsh/releases/latest/download/install.ps1).Content)) -FromRelease
```

ソースから入れる場合：

```sh
git clone https://github.com/sahenjp/rustdsh.git
cd rustdsh
./install.sh                 # ビルド＋ ~/.local/bin/rdsh に導入
./install.sh --as-dsh        # rdsh を `dsh` 名でも使えるよう置換（元は dsh-orig に退避）
./install.sh --restore       # 置換を元に戻す
./install.sh --prefix=DIR    # 導入先を変更（既定 ~/.local/bin）
```

install.sh は Linux / macOS / WSL 用です（WSL自動検出、cargoがなければ
rustupで自動導入。`--no-rustup` で無効化）。Windowsネイティブは install.ps1：

```powershell
git clone https://github.com/sahenjp/rustdsh.git
cd rustdsh
.\install.ps1              # ビルド＋ %LOCALAPPDATA%\rdsh\bin に導入（PATH追加つき）
.\install.ps1 -AsDsh       # `dsh` 名でも使えるよう置換（元は dsh-orig に退避）
.\install.ps1 -Restore     # 置換を元に戻す
.\install.ps1 -Wsl         # WSL側にも install.sh で連動導入
```

| OS | スクリプト | 備考 |
| --- | --- | --- |
| Linux / macOS | `./install.sh` | cargoかcurlが必要（rustup自動導入） |
| WSL | ディストロ内で `./install.sh` | 自動検出。ネイティブ併用は `install.ps1 -Wsl` |
| Windows（ネイティブ） | `.\install.ps1` | Rustが必要。コンパイルにMSVCビルドツールが必要 |

モデル未接続の初回起動は、DeepSeekプロンプトに置き去りにせず案内を出します：
`rdsh setup` を実行してください（`rdsh setup --login` ならCodex/opencodeの
OAuthフローをその場で起動します）。

ソースから直接ビルドする場合は `cargo build --release` で `target/release/rdsh` ができます。

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
rdsh search-web "rust async" --limit 5  # Web検索（SearXNG経由、既定 http://127.0.0.1:8888、`$SEARXNG_URL` で変更）
rdsh compact ./s.jsonl --max-tokens 8000 # セッションJSONLの圧縮（元ファイル不変）
rdsh sessions --limit 20 --tokens  # セッション一覧＋展開後トークン見積
rdsh logs --tail 50 --grep ERROR   # 起動ログの参照
rdsh profiles / rdsh skills        # プロファイル・スキル一覧
rdsh doctor                        # 本家dsh・DSH_HOME・slim設定の確認
rdsh bench --n 5                   # rdsh/dsh の起動比較
rdsh serve                         # Webダッシュボード（:3080）
```

### 選択した認証情報だけを共有する（rdsh auth）

Codex / OpenCode / 環境変数の認証情報を、秘密値なしで一覧にします。
共有元とcredentialを選ぶまでコピーしません。取込先はDSH本来の
$DSH_HOME/.credentials.yamlです。選択は秘密値を含まない
$DSH_HOME/rdsh-auth-sharing.jsonに保存します。

```sh
rdsh auth                                      # 共有元・保存先・選択状態・解除手順
rdsh auth --json                               # 秘密値を含まない機械可読の一覧
rdsh --dry-run auth --select codex:openai-codex --import # previewのみ、ファイル変更なし
rdsh auth --select codex:openai-codex --import    # CodexのOAuthだけ共有
rdsh auth --select opencode:openai-codex --import # OpenCodeのopenaiログインだけ共有
rdsh auth --select codex:OPENAI_API_KEY --import  # CodexのAPIキーは別に選択
rdsh auth --select env:DEEPSEEK_API_KEY --import  # この環境変数だけ永続保存
rdsh auth --unselect codex:openai-codex          # 将来の取込停止、既存コピーは保持
rdsh setup                                    # 選択済みの同期と初回案内
rdsh setup --web                              # 共有元・選択コマンド・個別キー保存
```

--select / --unselectは必要なものだけ個別に繰り返せます。選択すると
boot・dump-config・plugin・dsh名での委譲・setupでも、その共有元から
同期します。--importはすぐに取り込みます。別CLIの未選択トークンが
新しくても採用しません。DSH側の新しいgrantや既存API-key記録・ref、
他のエントリ、コメント、改行形式は保持します。Unixの認証ファイルと
選択ファイルは0600で保存します。

対象はCodexの ~/.codex/auth.json、OpenCodeの
$XDG_DATA_HOME/opencode/auth.json（未設定時はOSのdataディレクトリ）と
旧 ~/.config/opencode/auth.jsonです。OpenCodeのopenaiログインはDSHの
openai-codexへ写すので、選択名はopencode:openai-codexです。
環境変数の保存対象はDEEPSEEK_API_KEY、OPENAI_API_KEY、ANTHROPIC_API_KEYです。
一覧は形式・存在の確認であり、実際の認証成功を意味しません。

すでに渡した環境変数は、DSHとその子プロセスに引き継がれます。
この経路ではrdshはファイルを書かず、JSONにpersistent=false、
共有先、停止・失効手順を表示します。保存するなら別途選択してください。
setup --yesだけでは全環境キーを保存しません。対話setupやWeb画面での
個別の保存は引き続き可能です。

**利用範囲と解除:** 保存した内容は同じDSH_HOMEの全プロファイルで
使えます。選択解除は将来のrdshによる取込を止め、保存済みコピーと
実行中プロセスは残します。コピーの利用を止めるにはDSHを停止して
認証ファイル内の該当record/refを削除します。credential自体の失効は
providerでOAuth grantを取り消すかAPIキーを更新してください。
共有元CLIも使えなくなる場合があります。rdshによるコピーの制御であり、
DSH自身の更新・環境変数の継承・OS sandboxを制御するものではありません。

**従来版からの移行:** 既存のDSH認証情報は残します。選択ファイルが
なければ何も選択せず、従来のauth --importも選択を要求します。
RDSH_AUTH_AUTOSYNC=1で全取込へ戻ることはありません。一覧とpreviewで
確認して必要な共有元を選んでください。RDSH_AUTH_AUTOSYNC=0では
選択を残して自動同期だけを止めます。明示したauth --importは実行できます。
壊れた・読めない選択ファイルでは同期を停止します。

選択変更とrdshの認証書込は共通lockで直列化し、解除完了後に古い選択で
取り込むことを防ぎます。writerの異常終了でlockが残った場合は、
rdsh auth/setupの書込が動いていないことを確認してから
$DSH_HOME/.rdsh-auth-sharing.lockを除去してください。lock中は
launcherが取込をスキップします。

setup --webは毎回アクセス鍵を発行し、#key=...付きURLを表示します。
読み取り・キー保存・終了に同じ鍵が必要です。URLを他人に共有しないでください。
画面では保存前に保存先・利用範囲と、選択解除・コピー削除・provider失効の
違いを確認できます。

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
- 本家の探索順： `RDSH_ORIG_BIN`（旧 `DSH_ORIG_BIN` も有効）→ `~/.config/rdsh/origin` → 退避ファイル（dsh-orig等）→ PATH（自分を除外）→ このOS・CPUに合う最新の `~/.local/opt/node-v*` ツリー
- 命名は本家に準拠：コマンド・フラグはケバブケース（`dump-config`等）、`DSH_`環境変数名前空間は本家の所有とし、rdsh固有キーは `RDSH_` 配下に置きます
- 一時退避： `RDSH_PASSTHROUGH=1 dsh ...`（slim無し）、`RDSH_DRY_RUN=1 dsh ...`（実行内容のみ表示）
- slimは `NODE_COMPILE_CACHE` も付けます（Node 22.1以上のみ、利用者設定を優先、`RDSH_NODE_COMPILE_CACHE=0` で無効化）。本家dshは `RDSH_*` を読みません
- 既定プロファイル： `RDSH_DEFAULT_PROFILE` → ローカルの `tui` → 案内付きエラーの順（dsh 0.2.0に `tui` テンプレートはありません）
- 注意： `dsh tokens` のようにプロファイル名が予約語と衝突する場合は `dsh --profile tokens` で起動してください
- Nodeラッパー： `node "$(... dsh ...)"` 形式のスクリプトは置換中に壊れます（`dsh`はJSではなくネイティブバイナリのため）。`node`経由ではなく `dsh`/`rdsh` を直接実行してください。対象は `rdsh doctor` が一覧表示します

## Smart-DSH との併用

[Smart-DSH](https://github.com/hikarioyama/Smart-DSH)はDSHのwebプロファイル用プラグイン集
（モバイルUI・Web Push通知・Esc停止）で、競合バイナリではありません。rdshと共存できます。

```sh
rdsh doctor                                    # dsh版＋Smart-DSHバンドルも表示
rdsh --profile web --dump-config | grep notify-push   # 構成の読取確認
rdsh plugin --profile web add /path/to/dsh-notify-push  # dsh plugin と同じ
rdsh --profile web                             # slim env付きで起動（プラグインに影響なし）
```

併用時の注意点：

- ポート：dsh web GUIと`rdsh serve`は既定3080です。dsh webを3080のまま使い、
  `rdsh serve --port 38080` に分けます
- 置換時：`install.sh --as-dsh`後はSmart-DSHの補助スクリプトがPATH上の`dsh`を
  Rust製と誤認します。`dsh-orig`を使うか`DSH_PACKAGE_DIR`を指定します
- 対応バージョン：Smart-DSHはDSH `0.1.2-rc.1`基準です。`rdsh doctor`の版表示で差異を確認します

## Web UI（ダッシュボード）

```sh
rdsh serve
# → http://127.0.0.1:3080/ を開く（localhost のみ、読取専用API）
# ※ dsh web GUIと同ポートのため競合時は `rdsh serve --port 38080` 等を使ってください
```

| API | 内容 |
| --- | --- |
| `GET /api/version` | バージョン |
| `GET /api/doctor` | 状態確認 |
| `POST /api/tokens` | トークン推定（`{"text"}`） |
| `POST /api/prune` | 切り詰め（`{"text","max_tokens"}`） |
| `GET /api/bench?n=3` | 起動計測 |
| `GET /api/sessions?limit=20` | セッション一覧 |
| `GET /api/skills` / `/api/profiles` | 一覧 |

外部依存はありません（標準ライブラリのみ＋単一HTML埋め込み、CDN不要・オフライン可）です。

使い分け：`rdsh serve` は手元の簡易状態ページです。プロジェクトの指標・質問と回答・スマホ接続には、下の [Node.jsダッシュボード](dashboard/README.md) を使います。

### どちらを使うか（`rdsh serve` と `dashboard/`）

- 手元の状態確認（バージョン・doctor・tokens・sessions）だけなら `rdsh serve` を使います。`rdsh` バイナリだけで動きます。
- プロジェクトの指標・タスク・質問と回答・スマホ接続には `dashboard/` を使います。Node.js 22+ が必要です。詳しくは[Node.jsダッシュボードの案内](dashboard/README.md)を見てください。
- dsh web GUIと同ポート（3080）で競合したら、dsh webを3080のままにして `rdsh serve --port 38080` で分けます。

### プロジェクト専用ダッシュボードとスマホ接続

追加の [Node.jsダッシュボード](dashboard/README.md) では、費用・タスク・質問と回答を
プロジェクトごとに管理し、Tailscale経由のQRコードでスマホから開けます。
`rdsh-dashboard project --project <ディレクトリ>` と、元のHarness Web画面を起動する
`rdsh-dashboard harness` を分けて使います。ChatGPT Dots向けのMCP Eventsも備えています。
Node.js 22+が必要です。導入・MCP設定・Secure MCP TunnelによるDots接続は上記ガイドを参照してください。

## 安全設計

1. agent loop・profile bootの再実装はしません。`exec`委譲のみです
2. slimは**環境変数の追加だけ**です。本家が知らないキーは無視されます
3. `desktop`プロファイル拒否・dump排他など本家のエラー条件をRust側でも再現します
4. 読取系（tokens/search/compact/dump --native/serve API/inspect）は元ファイルを書き換えません
5. `--passthrough`・`RDSH_PASSTHROUGH=1`・`./install.sh --restore`で即時退避できます

### 検証（すべて実行済み）

- `cargo test`：単体・隔離CLI統合テスト（トークン計算・ワイルドカード・引数分割・auth系）
- `tests/regress.sh`：CLI回帰検査（全サブコマンド・異常系・auth取込往復・setup初回導線・dsh名委譲の隔離検証）
- 高速化の前後で出力をdiff比較し、完全一致を確認（300件search・上限打ち切りsearch）
- 実置換後に `dsh --version`（委譲）と `dsh guard`（新機能）を実機確認

## 高速化の仕組み

- トークン推定のASCII高速路：純ASCIIは `len/4` 一発計算（非ASCIIのみ従来走査、結果は同一）
- searchの二段階化：逐次walkで順序固定→ファイル単位で並列grep→walk順に結合。32ファイル未満は従来の逐次路のままです
- sessions --tokensの展開並列化：zstd展開をスレッド分散（数値は逐次と同一、順序保持）
- ビルドは `opt-level=z`＋LTO＋strip＋`panic=abort` で小型維持（約806KB）
- 再現： `python3` で9.6MBテキスト・300ファイル合成木を作り、新旧バイナリを `time` 比較（旧版はgit worktreeでHEADビルド）

## 構成

- `src/main.rs` — CLI定義・振り分け・`dsh`名検出
- `src/auth.rs` — 認証一覧と選択した共有元だけの取込（codex/opencode/環境変数→credentials.yaml）
- `src/dsh_args.rs` — 本家 `lib/bin.js` 互換の引数分割（読取専用）
- `src/passthrough.rs` — 本家探索＋`exec`委譲
- `src/slim.rs` — slim env定義
- `src/tokens.rs` — トークン推定・prune
- `src/search.rs` — 順序保持の並列grep
- `src/websearch.rs` — SearXNG Web検索（`search-web`、APIキー不要）
- `src/compact.rs` — JSONLセッション圧縮
- `src/inspect.rs` — sessions/logs/skills/profiles参照
- `src/guard.rs` — hooks.json用ガード
- `src/serve.rs`＋`src/ui.html` — ローカルWeb UI
- `src/setup_web.rs`＋`src/setup.html` — フローティングのセットアップUI（`setup --web`）
- `install.sh` — 導入（`--as-dsh`置換／`--restore`復元）
- `tests/regress.sh` — CLI回帰試験（CLI回帰検査）

## コミュニティ

- まず [CONTRIBUTING.md](CONTRIBUTING.md)（PRは4行、スクリーンショット規定）。
- バグ・要望：[Issueフォーム](https://github.com/sahenjp/rustdsh/issues/new/choose)（日本語OK）。
- 質問・相談：[Issues](https://github.com/sahenjp/rustdsh/issues)。
- 脆弱性は公開Issueに書かず [SECURITY.md](SECURITY.md) へ。
- 設計資料：[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)・
  [docs/BENCHMARKS.md](docs/BENCHMARKS.md)・[docs/ROADMAP.md](docs/ROADMAP.md)・
  [docs/RELEASING.md](docs/RELEASING.md)・[CHANGELOG.md](CHANGELOG.md)。
- 改善案の索引：[Issue #74](https://github.com/sahenjp/rustdsh/issues/74)（全72案と機能Issueの対応表、優先度P0-P3付き）。

## よくある質問

- **3080が使用中と言われる**：dsh web GUIと同ポートです。`rdsh serve --port 38080` を使ってください
- **プロファイル名がサブコマンドと被る**：`dsh --profile <name>` 形式で起動してください
- **元に戻したい**：`./install.sh --restore`（退避した本家を復元）
- **`--tokens` の `?` 付き表示**：zstd CLIが無い環境では圧縮サイズからの概算である印です

## クレジット

アイディア： [@studio_yebisu](https://x.com/studio_yebisu)、
[@remydre8](https://x.com/remydre8)。

## ライセンス

MIT（[LICENSE](LICENSE)）です。
