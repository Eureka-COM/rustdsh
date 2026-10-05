# Changelog

All notable changes to this project are documented here.
Format follows Keep a Changelog, versioning follows Semantic Versioning.

## [Unreleased]

### Added

- GitHub community health: Code of Conduct, Security/Support policy,
  issue forms, PR template, Dependabot, docs set.
- Docs: ARCHITECTURE / BENCHMARKS / ROADMAP / RELEASING guides.

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

[Unreleased]: https://github.com/sahenjp/rustdsh/compare/v0.1.2...HEAD
[0.1.2]: https://github.com/sahenjp/rustdsh/releases/tag/v0.1.2
[0.1.0]: https://github.com/sahenjp/rustdsh/tree/806c583
