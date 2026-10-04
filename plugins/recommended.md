# Recommended dsh plugins (not in `dsh-base`)

Install with `./plugins/install.sh` (defaults to the `headless` profile).
All entries below were verified present in the shipped dsh distribution and
absent from the `dsh-base` bundle, so each one adds real capability.

| Package | Why |
|---|---|
| `@deepseek-ai/dsh-tool-present` | Final deliverables as file cards (spreadsheets, decks, images) |
| `@deepseek-ai/dsh-tool-ask-user` | Ask the user for confirmation, choices, or missing info mid-run |
| `@deepseek-ai/dsh-tool-str-replace-editor` | Claude-Code-style `view`/`create`/`str_replace` editor tool |
| `@deepseek-ai/dsh-skill-office` | Word / PowerPoint / Excel creation, edits, and PDF conversion |
| `@deepseek-ai/dsh-hooks-claude-code` | Reuse your existing Claude Code `hooks.json` during agent runs |
| `@deepseek-ai/dsh-tool-bash-persistent` | Shell with cwd/env/jobs persisting across calls |
| `@deepseek-ai/dsh-mcp-client` | Use tools from external MCP servers (`mcp__<server>__<tool>`) |

Usage:

```sh
./plugins/install.sh                    # PROFILE=headless (default)
PROFILE=web ./plugins/install.sh       # install into the web profile instead
DRY_RUN=1 ./plugins/install.sh         # print the pnpm commands only
```
