# Architecture

`rdsh` is a fast Rust launcher for `dsh` (the DeepSeek Harness CLI).
Design rule: **port only the hot paths, delegate everything else**.

## Big picture

```
user -> rdsh (argv) -> decision
                          |- native fast path (no Node startup)
                          |    tokens / compact / inspect / serve / guard / search
                          |    auth / setup / doctor / profiles / skills / logs / sessions
                          `- passthrough: verbatim exec of dsh-orig
                               (agent loop and profile boot are never reimplemented)
```

- Delegation is a verbatim `exec`: zero behavior change by construction.
- Optimizations must be output-identical (see `tests/regress.sh`).

## Crate layout (`src/`)

| File | Role |
| ---- | ---- |
| `main.rs` | CLI definition (clap), dispatch, `--dry-run` / `--passthrough` handling |
| `dsh_args.rs` | dsh-compatible arg parsing, profile / patch / overlay handling |
| `passthrough.rs` | Verbatim `exec` delegation to the original `dsh` |
| `slim.rs` | Slim env for delegated boot (`NODE_COMPILE_CACHE`, node-tree lookup) |
| `tokens.rs` | Fast token estimation |
| `compact.rs` | Context compaction to a token budget |
| `search.rs` | Fast multi-file search (`estimate_tokens`) |
| `inspect.rs` | Config / session inspection |
| `serve.rs` | Status page server |
| `guard.rs` | Stdin command guard (`--deny` patterns) |
| `auth.rs` | OAuth state detection (`provider_needs()`), credential import |
| `setup_web.rs` + `setup.html` | First-run setup flow |
| `websearch.rs` | SearXNG-backed web search |
| `ui.html` | Floating setup UI |

## Optional Node dashboard (`dashboard/`)

Separate from the Rust launcher. Two modes, both loopback-bound:

- `project`: project metrics / tasks / Q&A via six project-scoped MCP tools.
- `harness`: launches the original Harness Web UI in a managed child process.

Phone access goes through Tailscale Serve (QR holds the access key).
See `dashboard/README.md`.

## Installers and services

- `install.sh` (Linux/macOS/WSL), `install.ps1` (Windows, incl. `-Wsl`).
- `sync-dsh.sh` + `systemd/rdsh-sync.*`: keep the upstream Harness in sync.
- `audit-rdsh.sh` + `systemd/rdsh-audit.*`: periodic audit hooks.
- `plugins/`: Smart-DSH compat bundle, update banner, skill installer.

## Invariants for contributors

1. Never reimplement the agent loop or profile boot.
2. Delegation stays byte-identical (`--passthrough` is the reference).
3. Every optimization ships with a before/after output diff.
4. `doctor` must stay truthful: wrappers, shadowing, and auth state.
