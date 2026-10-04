# rdsh — a fast, safe Rust launcher for `dsh`

[![ci](https://github.com/sahenjp/rustdsh/actions/workflows/ci.yml/badge.svg)](https://github.com/sahenjp/rustdsh/actions/workflows/ci.yml)
[![license](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![rust](https://img.shields.io/badge/rust-1.73%2B-orange.svg)](https://www.rust-lang.org/)

[日本語版](README.ja.md)

`rdsh` is a drop-in fast path for [dsh](https://github.com/deepseek-ai/deepseek-harness)
(the DeepSeek Harness CLI). Instead of a full rewrite, it **ports only the hot paths
to Rust and delegates everything else to the original `dsh` binary** — so you get
~81x faster startup and ~1/20th the memory with zero behavior change.

- Startup median **~1.19ms** (original `dsh`: ~96.7ms)
- Resident memory **~2.9MB** (original: ~66MB), single ~799KB binary, no runtime tree
- Safe by construction: agent loop and profile boot are never reimplemented,
  delegation is a verbatim `exec`, and every optimization is output-identical

## Contents

- [Benchmarks](#benchmarks)
- [Install](#install)
- [Usage](#usage)
- [Replacement mode (run as `dsh`)](#replacement-mode-run-as-dsh)
- [Web dashboard](#web-dashboard)
- [Safety design](#safety-design)
- [How it got fast](#how-it-got-fast)
- [Project layout](#project-layout)
- [Contributing](#contributing)
- [FAQ](#faq)
- [License](#license)

## Benchmarks

Measured on Linux x86_64, including before/after comparisons for the optimizations.

| Case | rdsh | Baseline | Factor |
|---|---|---|---|
| `--version` startup (median, n=5) | ~1.19ms | original `dsh` ~96.7ms | ~81x |
| `--version` peak RSS | ~2.9MB | original ~66MB | ~1/23 |
| Hook-equivalent peak RSS | ~2.7MB | equivalent Node script ~45MB | ~1/16 |
| search (300 files, ~600k lines) | ~12ms | before ~41ms | ~3.4x |
| tokens (9.6MB text) | ~12ms | before ~35ms | ~2.9x |
| sessions --tokens (20 sessions) | ~0.41s | before ~1.65s | ~4.0x |
| Distribution size | one ~799KB binary | ~508MB Node tree | — |

Reproduce with `rdsh bench --n 5` and `/usr/bin/time -v`. The before/after
binaries were built from HEAD vs. the working tree in a scratch worktree and
their outputs were diffed for equality.

## Install

```sh
git clone https://github.com/sahenjp/rustdsh.git
cd rustdsh
./install.sh                 # build + install to ~/.local/bin/rdsh
./install.sh --as-dsh        # also shadow `dsh` (original kept as dsh-orig)
./install.sh --restore       # undo the shadowing
./install.sh --prefix=DIR    # custom install dir (default ~/.local/bin)
```

Or build directly: `cargo build --release` produces `target/release/rdsh`.
Requires Rust 1.73+ (uses `u32::div_ceil`, `thread::scope`); only three
dependencies (`clap`, `serde_json`, `anyhow`), no async runtime, no build scripts.

## Usage

### dsh-compatible delegation

```sh
rdsh tui                          # same as: dsh --profile tui (with slim env)
rdsh --profile web --patch x.yml  # boot with an extra overlay
rdsh --passthrough tui           # byte-identical delegation, no slim env
rdsh --dry-run tui -- --resume abc  # print what would be executed
```

### Native fast commands (no Node startup)

```sh
rdsh tokens ./AGENTS.md               # estimate input tokens (~4 chars = 1, CJK = 1 each)
echo ... | rdsh prune --max-tokens 4000  # keep head+tail within a token budget
rdsh search TODO --dir . --max 100   # recursive grep (parallel, same order as sequential)
rdsh compact ./s.jsonl --max-tokens 8000 # compact a session transcript (source untouched)
rdsh sessions --limit 20 --tokens    # list sessions with decompressed token estimates
rdsh logs --tail 50 --grep ERROR     # inspect startup logs
rdsh profiles / rdsh skills          # list profiles and skills
rdsh doctor                          # check original dsh, DSH_HOME, slim setup
rdsh bench --n 5                     # compare rdsh vs dsh startup
rdsh serve                           # local web dashboard (:3080)
```

### `rdsh guard`: a fast hook command for hooks.json

`guard` scans stdin (hook JSON or raw text) for deny patterns and blocks on
match: exit code 2 with the reason on stderr, exit 0 otherwise. With `--json`
it prints `{"decision":"block"}` / `{"decision":"approve"}` instead. `*` in a
pattern matches any string. At ~1ms startup and ~3MB RSS, per-tool-call hook
cost is effectively zero.

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

This follows the dsh hook protocol: exit 2 blocks with a message the model sees,
any other failure is non-blocking and only logged.

## Replacement mode (run as `dsh`)

When the binary is invoked under the name `dsh`, anything that is not an
rdsh-native subcommand is delegated verbatim to the original binary, so
`dsh --version`, `dsh --profile tui`, and `dsh --help` stay byte-identical.

- Original-binary discovery order: `DSH_ORIG_BIN` → `~/.config/rdsh/origin` →
  sibling backups (`dsh-orig`, `dsh.orig`, `dsh.real`) → `PATH` (self excluded) →
  known npm install paths
- One-shot escapes: `RDSH_PASSTHROUGH=1 dsh ...` (no slim env),
  `RDSH_DRY_RUN=1 dsh ...` (print only)
- Name shadowing: a bare `dsh tokens` runs the rdsh subcommand; a profile
  literally named `tokens` still boots via `dsh --profile tokens`

## Web dashboard

```sh
rdsh serve
# open http://127.0.0.1:3080/ (localhost only, read-only API)
# if the port is taken (the dsh web GUI also uses 3080), try --port 38080
```

| API | Purpose |
|---|---|
| `GET /api/version` | version |
| `GET /api/doctor` | health check |
| `POST /api/tokens` | token estimate for `{"text"}` |
| `POST /api/prune` | prune `{"text","max_tokens"}` to budget |
| `GET /api/bench?n=3` | startup measurement |
| `GET /api/sessions?limit=20` | recent sessions |
| `GET /api/skills`, `/api/profiles` | name lists |

Dependency-free (std-only HTTP server plus one embedded HTML file, no CDN,
works offline).

## Safety design

1. The agent loop and profile boot are never reimplemented — delegation only.
2. Slim mode only *adds* environment variables; unknown keys are ignored upstream.
3. Launcher error cases from the original (`desktop` profile, mutually exclusive
dumps, missing `--profile`) are reproduced in Rust.
4. Read paths never write: tokens/search/compact/dump/native APIs touch nothing.
5. Instant retreats: `--passthrough`, `RDSH_PASSTHROUGH=1`, `./install.sh --restore`.

### Verification (all executed)

- `cargo test`: 14 unit tests pass (token math, wildcard matcher, arg splitter).
  The suite caught and fixed one real matcher bug (single-pattern substring).
- `tests/regress.sh`: 19 CLI checks pass (every subcommand, error paths, and
  sandboxed `dsh`-name delegation against a fake original).
- Optimization diffs: old vs. new binary outputs compared byte-for-byte
  (300-hit search and truncated-max search both identical).
- Live replacement verified on a real machine: `dsh --version` still delegates,
  new native commands work under the `dsh` name.

## How it got fast

- ASCII fast path for token estimation: pure-ASCII input is one `len/4`
  computation (non-ASCII keeps the exact scan; results identical).
- Two-phase search: sequential walk fixes the order, files are grepped in
  parallel, hits merge back in walk order. Trees under 32 files keep the exact
  old sequential code path.
- Parallel zstd expansion for `sessions --tokens` (same numbers, order kept).
- Release profile stays small: `opt-level=z`, LTO, `strip`, `panic=abort` (~799KB).

## Project layout

- `src/main.rs` — CLI definition, dispatch, `dsh`-name detection
- `src/dsh_args.rs` — original `lib/bin.js`-compatible arg splitter (read-only)
- `src/passthrough.rs` — original-binary discovery + `exec` delegation
- `src/slim.rs` — slim environment definition
- `src/tokens.rs` — token estimation and pruning
- `src/search.rs` — order-preserving parallel grep
- `src/compact.rs` — session transcript compaction
- `src/inspect.rs` — read-only sessions/logs/skills/profiles views
- `src/guard.rs` — hooks.json guard command
- `src/serve.rs` + `src/ui.html` — local web dashboard
- `install.sh` — installer (`--as-dsh` shadow / `--restore`)
- `tests/regress.sh` — CLI regression suite (19 checks)

## Contributing

```sh
cargo fmt --check      # must be clean
cargo clippy --all-targets -- -D warnings   # must be clean
cargo test             # 14 unit tests
BIN=./target/debug/rdsh sh tests/regress.sh # 19 CLI checks (needs cargo build first)
```

No new dependencies without discussion: binary size and startup time are
features. Behavior changes must extend `tests/regress.sh`.

## FAQ

- **Port 3080 is busy?** The dsh web GUI uses it too — run `rdsh serve --port 38080`.
- **A profile collides with a subcommand name?** Boot it explicitly:
  `dsh --profile <name>`.
- **Revert the replacement?** `./install.sh --restore` brings the original back.
- **What does `~123tok?` mean?** Without the `zstd` CLI the estimate falls back
  to compressed-bytes/4; the `?` marks that.

## License

MIT — see [LICENSE](LICENSE).
