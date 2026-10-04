# rdsh — dsh の Rust 高速ランチャー（安全な移植）

フル移植ではなく **ホットパスだけ Rust 化 + 残りは本家 dsh に委譲**。
性能低下なし・システム破壊なしが設計の前提。

## Web UI（ダッシュボード）

```sh
rdsh serve --port 8080
# → http://127.0.0.1:8080/ を開く（localhost のみ、読取専用API）
```

- 状態確認（doctor）、トークン推定、prune、起動ベンチをブラウザから実行
- API: `GET /api/version` `GET /api/doctor` `POST /api/tokens` `POST /api/prune` `GET /api/bench?n=3`
- 依存なし（std のみ + 埋め込み単一HTML、CDN 不要・オフライン可）

## なぜ速い・軽い・トークン節約になるか

| 施策 | 効果（実測） |
|---|---|
| `rdsh --version` ネイティブ（Node 起動なし） | 起動 median **約1ms**（本家 dsh 約90ms の **約90倍**） |
| RSS **約2.6MB**（本家約66MB の約1/25）・バイナリ **709KB**（Web UI 同梱後） | メモリ・CPU 削減 |
| `exec` で本家に置換（子プロセスを残さない） | 常駐メモリ +0 |
| `tokens / prune / search / compact` をネイティブ化 | 初期入力トークン削減、Node fs-search 回避 |
| slim env（`DSH_SLIM=1` 等、未知キーは本家が無視） | 重い任意バンドル（voice / auto-review）抑止のヒント |

## 使い方（dsh 互換）

```sh
rdsh tui                        # = dsh --profile tui（slim ON で委譲）
rdsh --profile web --patch x.yml
rdsh --passthrough tui           # slim 無しの完全委譲（非常口）
rdsh --dry-run tui -- --resume abc  # 実行内容だけ表示

rdsh serve --port 8080          # Webダッシュボード
rdsh dump-config --profile tui --native  # Node なしで層一覧
rdsh tokens ./AGENTS.md
echo ... | rdsh prune --max-tokens 4000
rdsh search TODO --dir . --max 100
rdsh compact ./session.jsonl --max-tokens 8000
rdsh doctor
rdsh bench --n 5
```

## 安全設計（破壊しない理由）

1. agent loop・profile boot の再実装はしない。`exec` 委譲のみ。
2. slim は**環境変数の追加だけ**。本家が知らないキーは無視される。
3. `desktop` プロファイル拒否・dump 排他など本家エラー条件を Rust 側でも再現。
4. 読み取り系（tokens/search/compact/dump --native/serve API）は元ファイルを書換えない。
5. `--passthrough` でバイト等価委譲に即時退避できる。

## ビルド

```sh
export PATH="$HOME/.cargo/bin:$PATH"
cargo build --release  # target/release/rdsh
```

## 構成

- `src/main.rs` — CLI 定義・振り分け
- `src/dsh_args.rs` — 本家 `lib/bin.js` 互換の引数分割（読取専用）
- `src/passthrough.rs` — 本家 dsh 探索 + `exec` 委譲
- `src/slim.rs` — slim env 定義
- `src/tokens.rs` — トークン推定・prune
- `src/search.rs` — std のみ再帰 grep
- `src/compact.rs` — JSONL セッション圧縮
- `src/serve.rs` + `src/ui.html` — ローカル Web UI（std のみ HTTP）
