# Contributing (寄稿ガイド)

English summary first, 日本語の詳細は後に続きます.

## Quick start

```sh
git clone https://github.com/sahenjp/rustdsh.git
cd rustdsh
cargo build
cargo test
```

Full test matrix before a PR:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --release
sh tests/regress.sh
(cd dashboard && npm ci && npm test)   # if you touched dashboard/
```

## Pull Request

- PR body is 4 lines: overview / changes / verification / caveats.
- All green before opening: fmt, clippy (zero warnings), tests, regress.
- Behavior changes need a before/after output diff attached.
- Keep PRs small and single-topic; hot-path changes need benchmark numbers
  (see `docs/BENCHMARKS.md`).

## UI changes (required)

- Attach real before/after PNG screenshots. Text-only descriptions are not enough.
- Animated flows: add a short GIF plus PNGs for steps a GIF cannot show.
- Screenshots must be real captures, never mockups.
- Add measured numbers for anything performance-related.

## Architecture rules

- Never reimplement the agent loop or profile boot (see `docs/ARCHITECTURE.md`).
- Delegation stays byte-identical; `--passthrough` is the reference behavior.
- `doctor` must stay truthful about wrappers, shadowing, and auth state.

## Issues

Use the issue forms (bug / feature / performance / docs). Include:
`rdsh --version`, `rdsh doctor`, OS/shell, repro steps.

For the full proposal backlog, see the index at
[issue #74](https://github.com/sahenjp/rustdsh/issues/74)
(all 72 proposals mapped to feature issues, priority P0-P3).

## Branch protection proposal (`main`)

This is a proposal memo (settings need admin). See
[issue #11](https://github.com/sahenjp/rustdsh/issues/11).

- Require PRs for `main`: no direct push, no force push, no deletion.
- Require at least one approval from someone other than the PR author.
- Require status checks (lint, Rust tests per OS, dashboard tests) to pass.
- Dismiss stale approvals when new commits are pushed.
- Keep bypass permissions minimal; review collaborator `write` access
  (fork-based PRs are enough for code-only contributors).

---

## 日本語

### main の保護提案メモ

設定自体は管理者権限が必要です。詳しくは
[Issue #11](https://github.com/sahenjp/rustdsh/issues/11)を見てください。

- `main` への直接push・force push・削除を禁止し、PR経由にします。
- PR作成者以外の承認を最低1件必須にします。
- lint・Rust各OSテスト・dashboard各OSテストの成功を必須にします。
- 追加コミットで古い承認を無効化します。
- bypass権限は最小化し、協力者のwrite権限も見直します。

### Pull Request

- 本文は4行で書きます（概要／変更／検証／注意点）。
- `cargo fmt --check`、`cargo clippy --release --all-targets`（警告ゼロ）、
  `cargo test --release`、`tests/regress.sh` の全通過を確認してから出します。
- 既存の動作を変えるときは、新旧の出力差分を添えてください。

### UIを変えるときの約束

- 見た目の変更は、必ずPNG画像をPRに添付します。文言だけの説明は不可です。
- 変更前と変更後の2枚を並べるのが基本です。
- 動きのある変更（遷移・アニメ・操作手順）は、短いGIFも付けます。
  GIFだけでは追えない箇所はPNGを併用します。
- 画像とGIFは実際に動かした画面の撮影にし、モックや想像図は混ぜません。
- 応答速度など数値で示すべきものは、計測値も一緒に書きます。
