# rdsh-update-banner

Update notification banner for [rdsh](https://github.com/sahenjp/rustdsh) on the dsh web GUI.

When `sync-dsh.sh` (shipped with rdsh) records an update in
`~/.local/share/rdsh/update-state.json`, a notification card appears at the
top of the web GUI. From the card you can run the update check, see details,
minimize it, or dismiss it (dismissed versions stay dismissed).

## Install

```sh
dsh plugin --profile web add rdsh-update-banner
```

Then add one block to your web profile `cordis.patch.yml` (or run
`install-banner.sh` from the rdsh repo, which does both steps):

```yaml
- insert:
    - id: rdsh-update-banner
      name: "rdsh-update-banner"
      config:
        demo: false
```

Reload the web GUI to pick it up.

## Config

- `demo: true` shows a demo notification without touching any state file.
- Without updates recorded, the banner stays hidden.

## License

MIT
