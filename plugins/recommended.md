# Recommended filesystem skills (rtk + ponytail)

Install with `./plugins/install-skills.sh` (defaults to `~/.dsh/skills`).

| Skill | Source | Notes |
|---|---|---|
| ponytail + 5 companions | `DietrichGebert/ponytail@main` | shallow clone, `FORCE=1` refreshes with timestamped backup |
| rtk | local `rtk` binary (want 0.46.0+) | verified by the script; its `SKILL.md` is kept as-is |

```sh
./plugins/install-skills.sh                  # into ~/.dsh/skills
DSH_HOME=/tmp/test sh plugins/install-skills.sh  # sandbox trial
FORCE=1 ./plugins/install-skills.sh          # refresh ponytail from upstream
DRY_RUN=1 ./plugins/install-skills.sh        # preview only
```
