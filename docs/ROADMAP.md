# Roadmap

Direction, not promises. Items move when someone sends a PR.

## Now

- Keep delegation byte-identical while upstream `dsh` evolves.
- Keep installers working on Linux / macOS / WSL / Windows.
- Dashboard hardening: auth boundary, event subscriptions, QR flow.

## Next

- Routing experiments (see `docs/proposals/`): cheapest-route selection
  across subscription providers, starting with a 2-provider manual table.
- More `doctor` checks for broken wrappers and shadowed binaries.
- Release automation: checksums, SBOM, musl builds for sync.

## Later

- Windows-native dashboard parity with the WSL flow.
- Startup and memory regression tracking in CI (fail on >10 percent slip).
- i18n: keep README.ja.md in sync with README.md.

## Non-goals

- Reimplementing the agent loop or profile boot in Rust.
- Forking the Harness UI or its protocol.
- Supporting all providers at once in routing (one at a time).
