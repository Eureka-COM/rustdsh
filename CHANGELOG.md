# Changelog

All notable changes to this project are documented here.
Format follows Keep a Changelog, versioning follows Semantic Versioning.

## [Unreleased]

## [0.1.3] - 2026-10-05

### Added
- Context engine prototype: `rdsh context build/search/status/explain`
  rebuilds per-turn context from local files (no vector DB).
- Unified settings: `rdsh settings show/path/init` backed by
  `$DSH_HOME/rdsh.json`; every native default (budgets, limits, ports,
  guard patterns, default profile, beta flags) follows settings unless
  the CLI flag is passed explicitly.
- DSH settings UI: `plugins/rdsh-settings` adds an `rdsh` section to
  the settings sidebar (same design, order 20).
- Beta gate: `rdsh context` requires `beta.context_engine=true`.

## [0.1.2] - 2026-10-04

### Added
- Private project dashboards, QR access, and MCP Events.
- Single installer uploader; pinned line endings for installers.

### Fixed
- CI fixes around OAuth auto-recognition, first-run setup, release installers.

## [0.1.0] - 2026-09

### Added
- Initial Rust launcher: hot-path port with verbatim exec delegation to dsh.
- Native fast commands: tokens / compact / inspect / serve / guard / search.
- Floating setup UI, dsh-default detection, SearXNG search.
- Installers: install.sh (Linux/macOS/WSL), install.ps1 (Windows).

[Unreleased]: https://github.com/sahenjp/rustdsh/compare/v0.1.3...HEAD
[0.1.3]: https://github.com/sahenjp/rustdsh/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/sahenjp/rustdsh/releases/tag/v0.1.2
[0.1.0]: https://github.com/sahenjp/rustdsh/releases/tag/v0.1.0
