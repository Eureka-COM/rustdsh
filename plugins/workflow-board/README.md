# Workflow member board for rustdsh

This directory packages the read-only DSH workflow board as a verified source patch, with an executable preparation tool. It follows rdsh's existing separate plugin/install tooling while keeping the launcher and execution engine unchanged. It does not register another `workflow-run` plugin.

The patch changes only 18 DSH UI, test, documentation and lockfile paths. Its base is `f97c0438fb1608bbc4c08c88a27344249795ea22` (source package `0.1.7-rc.2`), and its board commit is `7bd9ac31c23fe369d1fa9a6849869f04967d6ee5`. `manifest.json` pins the patch checksum, commits and file set. No unrelated DSH source or history is vendored. DSH's MIT notice is retained in `DSH-LICENSE`; ZCode source is not copied.

## Prepare a compatible source checkout

Node.js 24 and Git are required. Select a clean, isolated DSH source checkout at the verified base. For example, clone `sahenjp/deepseek-harness` into a separate directory and detach it at the exact base above. Do not select the global npm package, an installed profile, or another task's checkout.

From the rustdsh checkout:

```sh
node plugins/workflow-board/source-patch.mjs --source ../dsh-board-source --check
node plugins/workflow-board/source-patch.mjs --source ../dsh-board-source --apply
```

The same commands work in PowerShell. Check is the default and writes nothing. Apply requires an explicit source root, the exact base, an intact patch, the expected file set and a clean working tree. It runs Git's complete-patch check before writing, verifies the result with a reverse check and reports `applied`. Repeated application reports `already-applied`. Wrong revisions, dirty checkouts, conflicts, changed patch bytes and installed-package directories are rejected with JSON diagnostic codes; existing files are preserved on preflight failure.

Build the patched DSH source using its normal workspace procedure, then choose explicitly how to install that build. This tool never installs packages, runs build scripts, replaces the installed DSH, changes profiles, or starts/stops tasks. rdsh continues to delegate to the chosen original DSH. An upstream update or different source revision needs a newly reviewed patch; the tool deliberately refuses to guess compatibility.

## What the board displays

The existing durable `workflow-run` renderer/reducer gains an in-chat toggle and a tab in the existing right sidebar. Cards represent started members, identified by runId plus member.seq. Columns show running, completed, failed, cancelled and interrupted; labels, exact phases and successful completions/started counts come from durable records. It does not invent model names, queued tasks, report counts, results or artifacts. Child navigation retains the parent's direct-child catalog and running checks. The selected Chat key is saved per Session, and facts remain in the existing live projection. Narrow layouts, collapse, Escape and focus preservation are supported.

## Validation and current limits

```sh
node --test plugins/workflow-board/tests/source-patch.test.mjs
```

The preparation tests use temporary Git fixtures and require no network, model or profile. CI runs them on Linux, macOS and Windows. They cover read-only preflight, actual application, idempotence, paths with spaces, dirty/untracked source, wrong roots/revisions, checksum/scope failures, multi-file conflicts without partial application, and installed-package rejection.

The bundled board source passed 49 focused DSH tests and real Chrome fixture QA at 1440px/390px, in light/dark themes, including reload, Session switching, live status movement, focus, Escape, permitted links and escaped text. These are production components driven by scripted events, not a connection to an installed DSH server. Actual installation/replacement, real-server workflow use and the complete DSH build/GUI/web gates remain unverified. The previously installed DSH `0.2.0-rc.2` shares the relevant public event interfaces but is not this source base; it is not automatically patched. See related [rustdsh issue #83](https://github.com/sahenjp/rustdsh/issues/83).
