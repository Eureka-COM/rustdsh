# Project dashboard and private Harness access

`rdsh-dashboard` provides two separate entry points:

| Mode      | Purpose                                                    | Data source                                      |
| --------- | ---------------------------------------------------------- | ------------------------------------------------ |
| `project` | Project metrics, tasks, questions, human answers, progress | Twelve project-scoped MCP tools                  |
| `harness` | Launch and open the original DeepSeek Harness Web UI       | A separately managed `dsh --profile web` process |

Both bind to loopback and can use **Tailscale Serve** for private HTTPS access.
The QR code includes the access key and opens the correct authenticated page.
Treat it as a credential; share it only with intended users on your tailnet.
Keep Tailscale connected on the PC and phone and keep the dashboard running.

Project 画面の QR 鍵はブラウザー専用です。画面は鍵を URL fragment から
そのタブの sessionStorage に保存し、API へヘッダーで送ります。MCP 設定の鍵と
管理用の鍵は別々で、MCP 鍵から人間の回答は登録できません。Harness は元の Web
画面との互換性のため Cookie を使いますが、その鍵で MCP や管理 API は使えず、
中継時には Harness 側へ渡しません。同じホスト上の別ポートにも Harness Cookie
が届くため、そのホスト上で信頼できない Web サービスを開かないでください。

This optional Node.js component is separate from the Rust launcher's `rdsh serve`
status page. It does not replace the original Harness agent loop.

## Install and launch

Requires Node.js 22+, and PowerShell 7 for Windows launchers. Tailscale is optional
for local use and must be installed and signed in for phone access.

```powershell
# From the repository root; installs dependencies and a launcher in ~/.local/bin.
pwsh -NoProfile -File ./dashboard/install-windows.ps1

# A project-specific dashboard. The default port is stable for each project path.
rdsh-dashboard project --project C:\Projects\MyProject --open

# A separate Harness instance, with front-end :38081 and backend :3081.
rdsh-dashboard harness --open

# Reopen an existing instance or stop only that instance.
rdsh-dashboard open --project C:\Projects\MyProject
rdsh-dashboard stop --project C:\Projects\MyProject
rdsh-dashboard open --harness
rdsh-dashboard stop --harness
```

The checkout must stay in place: the installed launcher points to its `cli.mjs`.
To run without installing a launcher, use `node dashboard/cli.mjs ...`.
On Linux/macOS, install with `npm ci --prefix dashboard` and use the Node CLI.
Project mode is portable. The current Harness launcher targets Windows + WSL;
its distribution defaults to `FlashNext`, with `/root/.local/bin/rdsh-env` as the
existing environment wrapper. Override those with `RDSH_WSL_DISTRO` and
`RDSH_WSL_HARNESS_BIN` when your WSL installation uses different names/paths.
It never stops an independently running Harness instance.

Use `--port 38100` to choose a project port, `--harness-port 3081` for its Harness
backend, or `--no-tailscale` for local access only. Conflicting ports fail startup.
Only one writer can run for a given canonical project directory.

### QR access

Open the dashboard and select **スマホで開く**. If Tailscale requests its first HTTPS
configuration, follow **利用設定を開く**, then select **接続を更新**. The QR code appears
only after the route has been verified. Harness QR codes open the actual Harness
UI, while project QR codes open the project's metrics and questions.

Existing Serve routes are preserved. A conflicting route or public Funnel on the
chosen port is rejected. Stop closes the local server and managed Harness child;
the private Serve route remains configured for the next launch.

## Report project data through MCP

State is stored outside the checkout:

- Windows: `%LOCALAPPDATA%\rdsh\dashboard\projects\<project-id>`
- Linux/macOS: `~/.local/state/rdsh/dashboard/projects/<project-id>`
- Override for isolated environments: `RDSH_DASHBOARD_HOME`

`project-id` is derived from the canonical directory path. Separate projects have
separate state, credentials, MCP endpoints, and event subscriptions.

Each running project writes `mcp-config.json` for stdio clients and
`mcp-http-config.json` for HTTP clients into that private directory. Import the
appropriate configuration into your agent's MCP settings. The dashboard must be
running before the stdio bridge is used. The stdio bridge reads the current
runtime credential on each operation. HTTP clients must reload the generated
configuration after a restart, because the bearer key rotates.
`runtime.json` の `token` は管理用、`mcp_token` は MCP 用です。ブラウザー用の
鍵は `browser_url` に含まれます。これらを別用途で使い回さないでください。

| Tool                       | Effect                                       |
| -------------------------- | -------------------------------------------- |
| `dashboard_update_metrics` | Report measured cumulative snapshots         |
| `dashboard_upsert_task`    | Create/update a task by ID                   |
| `dashboard_ask_question`   | Ask a human a question with a unique ID      |
| `dashboard_publish_event`  | Report progress or an artifact reference     |
| `dashboard_get_feedback`   | Read durable answers after a sequence cursor |
| `dashboard_get_state`      | Read this project's current state            |
| `dashboard_check_task_contract` | Preflight declared repo/cwd/write paths against a versioned task contract |
| `dashboard_check_operation` | Parse/audit explicit tool attributes; keep execution on hold |
| `dashboard_request_approval` | Create a pending request bound to one operation and its limits |
| `dashboard_check_approval` | Match a human grant to the current operation and next attempt |
| `dashboard_claim_approval` | Atomically reserve one approved attempt and declared cost |
| `dashboard_check_worker_start` | Resolve requested permissions and hold startup without an enforcing adapter |

Example tool arguments:

```json
{
  "id": "M3.6",
  "title": "Validate the host launcher",
  "status": "doing",
  "milestone": "M3"
}
```

```json
{
  "id": "Q1",
  "question": "Which result should we adopt?",
  "urgency": "high",
  "default_action": "Wait for a reply"
}
```

A person answers in the dashboard. The answer is persisted before notification.
`dashboard_get_feedback({"after":0})` returns `messages` and `next_cursor`; pass
the returned cursor on the next read. Reads never consume answers. The default
action is descriptive and is never executed by this server.

Unknown metrics display **未取得**. This component does not scrape billing,
estimate spend, or infer context loss. Report measured values using:
`total_cost_usd`, `total_budget_usd`, `session_cost_usd`, `session_budget_usd`,
`session_id`, `input_tokens`, `cached_input_tokens`, `model_calls`, `tool_calls`,
`tool_errors`, `context_misses`, `auto_continues`, `refusals`, `api_errors`.
Omitted fields retain their previous value; `null` clears a numeric field.
Cache read percentage is cached input tokens / input tokens. Tool error rate is
tool errors / tool calls. Costs are reported API-equivalent values, not invoices.

Artifact references are displayed as text. Local file contents are never opened
or served. Treat all user-authored questions, answers, progress, and paths as data.

## Versioned task contracts (P0 foundation, Issue #12)

Create the task first, then save its contract with the local administrator CLI:

```sh
rdsh-dashboard set-contract --project /path/to/repo --file contract.json
```

On Windows, use absolute native paths (JSON backslashes must be escaped).
The dashboard must already be running. This command uses its local administrator
credential internally; do not copy the credential into the contract or arguments.
Use test directories and `--no-tailscale` for local validation.

```json
{
  "task_id": "M3.6",
  "expected_version": 0,
  "purpose": "Validate the launcher locally",
  "repository": "/path/to/repo",
  "allowed_scope": "Edit source and local tests only",
  "write_roots": ["/path/to/repo/src", "/path/to/repo/tests"],
  "forbidden_actions": ["No remote push, publication, or credential changes"],
  "completion_conditions": ["Affected tests pass and the change is reviewed"],
  "change_reason": "Initial task scope"
}
```

`expected_version` is zero for creation and the current version for an update.
The server serializes updates and rejects a stale version. All versions, change
reasons and timestamps remain in project state; task reports, questions, answers
and conversation summaries never replace the contract. The task row displays the
current version and its history. MCP clients read the same history with
`dashboard_get_state`, including after restart. Existing schema-1 state remains
readable; tasks without contracts are shown as unset and preflight blocks them.

Contract versions are retained without a count limit and each update rewrites the
state file. Large histories increase storage and update cost. Automatic truncation
would discard the required scope-change audit; plan an archival/migration design
before adopting frequent contract revisions. Approval request/decision histories
are also retained without automatic truncation; recent check logs are bounded.

Only `POST /api/contracts/update` with the local administrator bearer credential
can save a contract. Browser and MCP credentials cannot change it; there is no
MCP contract mutation tool. This is the existing single-owner credential boundary,
not proof of a particular human's identity. A process running as the same OS user
may still read the private runtime file. Each change marks previous approvals as
requiring revalidation; no answer is converted to an execution approval. The
approval ledger below implements declared scope/retry matching; execution adapter
integration remains required.

Before an operation, call `dashboard_check_task_contract` with:

```json
{
  "task_id": "M3.6",
  "contract_version": 1,
  "repository": "/path/to/repo",
  "cwd": "/path/to/repo",
  "write_paths": ["src/main.rs"]
}
```

The corresponding HTTP endpoint is `POST /api/contracts/check`. `within_scope`
(HTTP 200) means only that these declared paths passed preflight. `block` or
`unparsed` (HTTP 409) must stop the caller. The stdio and HTTP MCP routes return
the same structured decision. Checks do not write state or read file contents.
The read-only check intentionally accepts administrator, MCP and human browser
project credentials, matching access to project state; unauthenticated requests
are rejected. Browser access does not grant contract mutation or operation execution.
They require the active version and project repository, check cwd containment,
and check both lexical and resolved write destinations. Empty `write_roots` means
no writes. Roots must exist when saved; a new write leaf may be absent. Escaping,
retargeted and dangling symlinks/junctions fail closed. Parent traversal and
ambiguous Windows device/ADS/drive-relative paths are unsupported. Diagnostics
return fixed reason codes without echoing supplied paths or command text.

This is an opt-in preflight boundary, not an execution grant or OS sandbox.
`allowed_scope`, `forbidden_actions` and `completion_conditions` are descriptive;
they are saved and displayed but are not parsed as executable policy. Commands,
read paths, network destinations and shell syntax are not evaluated; supplying
unknown fields returns `unparsed`. The existing launcher and original DSH tool
execution do not call this check automatically. Adapter integration, structured
tool/command policy (Issue #29), approval matching (Issue #30), input provenance
(Issue #32) and actual sandbox enforcement (Issue #33) are needed before expanding
execution permissions. Filesystem or contract changes after preflight can
invalidate a result; recheck at use, and rely on an execution sandbox for enforcement.

### Structured operation policy (Issue #29 foundation)

Contracts may add `operation_policy` with `schema: 1`, `read_roots`, `executables`
and `network_origins`. Omission grants no read/command/network rights in this
structural layer. Write roots remain the contract's `write_roots`. Read roots must
be existing directories inside the project. Each executable rule has an absolute
existing `file` and an exact `args` array; the stored rule retains the argument
count and SHA-256 digest, without raw argument values. Network rules are exact
HTTPS origins without credentials, paths, queries or fragments.

`dashboard_check_operation` (or `POST /api/policy/check`) accepts task ID, contract
version, repository and this explicit operation envelope:

```json
{
  "schema": "rdsh.operation.v1",
  "tool_name": "file.write",
  "tool_input": {"cwd": "/path/to/repo", "path": "src/new.txt", "content": "fixture"}
}
```

Supported tool input schemas are:

| Tool | Required input fields |
| --- | --- |
| `file.read` | `cwd`, `path` (existing regular file) |
| `file.write` | `cwd`, `path`, `content` (at most 64 KiB) |
| `process.exec` | `cwd`, `executable` (absolute file), `args` (exact direct argv) |
| `network.request` | `cwd`, `url` (HTTPS), `method`, `body` (at most 64 KiB) |

Unknown tools/fields, raw shell commands, recognized shell executables, ambiguous
paths and unresolved inputs return `unparsed`. This does not infer a tool's
effects from words in its text. Path, cwd, executable/argv and origin mismatches
return `block`. Only supported attributes inside the current policy return
`within_policy`; execution still returns `hold`. These are rdsh's declared input
schemas, not claims that external CLI tools are already adapted.

The latest 1,000 checks are durable and displayed separately from the contract.
Reasons are fixed codes; bodies, URL queries and raw argument values are omitted
from audit state. Digests bind data without duplicating it; hashes are not secret
storage or encryption. File metadata and real paths are inspected, file contents
are never read. The existing Rust pattern guard remains independent and is not
called by this layer. Responses explicitly identify pattern as `not_evaluated`,
structure as the decision, and enforcement as `not_applied`. No declared command
or network operation is executed. Automatic CLI tool interception, shell
semantics, dynamic subprocess effects and OS enforcement remain unconnected.

### Approval ledger (Issue #30 foundation)

General question answers remain feedback. They never grant operation approval.
Create a request with `dashboard_request_approval` or `POST /api/approvals/request`:

```json
{
  "id": "request-1", "expected_version": 0,
  "task_id": "M3.6", "contract_version": 1, "repository": "/path/to/repo",
  "run_id": "run-1", "command_id": "command-1",
  "operation": {
    "schema": "rdsh.operation.v1", "tool_name": "file.write",
    "tool_input": {"cwd": "/path/to/repo", "path": "src/new.txt", "content": "fixture"}
  },
  "limits": {"max_cost_usd": 1, "max_attempts": 2},
  "expires_at": "2026-10-06T00:00:00.000Z",
  "source_ref": "request-artifact:command-1"
}
```

Replace the example expiry with a future UTC ISO timestamp within seven days.
Only an operation already within the current structured policy can be requested.
Each request version records project/task/contract, run/command, resolved target,
operation/data digest, limits, expiry and a reference to the original request.
The reference is untrusted display data; inspect the original request before
granting. Raw operation bodies, URL queries and argv values are not copied to the
ledger. Revisions require `expected_version` and always start pending; old versions
remain readable and their grants cannot authorize the new version.

Use the browser's dedicated approval buttons to grant, reject or revoke that
request version. Only the human browser credential can call
`POST /api/approvals/decide` with `id`, `request_version`, `decision`.
MCP and administrator credentials cannot grant approval; generic update routes
cannot bypass this boundary. The approver is the authenticated `dashboard_owner`
role, not a claim of a named person's identity. Decision history is retained.

Check or claim with the same task/repository/run/command/operation plus `id`,
`request_version`, `attempt` (starting at one) and `cost_usd`. A check reserves
nothing. A claim serializes revalidation and reservation: only the next attempt
within `max_attempts` and the cumulative declared cost is accepted. Costs have
micro-dollar precision and are declarations, not measured billing. Changed
scope/data/context, expired/revoked/rejected/pending requests and replayed attempts
fail closed. Use a new request version and obtain a new grant for changed limits
or operation data. Latest checks and all request versions survive restart.

HTTP check/claim returns 200 for `approval_valid`, otherwise 409; MCP returns the
same structured result. A valid reservation **still returns `execution: hold`**
and `enforcement: not_applied`; its use record is `execution: not_started`.
There is no operation execution, automatic refund, billing integration or DSH
tool interception. An enforcing execution adapter must perform a fresh check and
bind actual effects/cost before this ledger can grant effective permissions.

### Unsupported execution environments (Issue #33 foundation)

`dashboard_check_worker_start` (`POST /api/workers/check`) accepts `task_id`,
`contract_version`, `repository`, `run_id`, `worker_role` (`review` or
`implementation`). It resolves the current contract into **requested**
permissions. Review workers request read roots with no writes, executables or
network. Implementation workers request only the contract's declared roots,
exact command/argv rules and network origins. Callers cannot supply wider roots,
capability flags, a sandbox identity or an approval to bypass this check.

There is currently **no registered OS/container enforcing adapter**. Every valid
request returns `decision: hold`, `reason: enforcement_adapter_unavailable`,
`execution: not_started`, `effective_permissions: null` and `fallback: disabled`.
HTTP returns 409; MCP returns the same structured hold. This function starts no
worker and changes no ACL, credential, firewall, container or system setting.
Unparsed inputs and stale/outside contracts cannot start a worker either.

An adapter must prove filesystem read/write restrictions, network and exact
command limits, child-process inheritance, link escape protection and race-safe
access before effective permissions can be reported. Realpath preflight cannot
detect hardlink aliases or mounted filesystems, and can race with filesystem
changes. The original DSH launcher remains independent and does not call this
gate. Real OS refusal tests and launcher interception are still required; the
isolated source tests establish the unsupported-environment hold only.

## OpenAI ChatGPT Dots and MCP Events

The project HTTP `/mcp` endpoint implements MCP 2.0 (`2026-07-28`) discovery and
native `events/list`, `events/subscribe`, and `events/unsubscribe`, with the same
project bearer authentication used for tools. Available events:

- `dashboard.answer.created`
- `dashboard.question.created`
- `dashboard.task.updated`
- `dashboard.progress.updated`
- `dashboard.metrics.updated`
- `dashboard.contract.updated`
- `dashboard.policy.checked`
- `dashboard.approval.requested`
- `dashboard.approval.decided`
- `dashboard.approval.checked`
- `dashboard.approval.claimed`

Each requires the project's `project_id` filter. Subscribe to
`dashboard.answer.created` when a Dot should react to human replies, then retrieve
the full answer with the feedback tool. Users choose what the Dot should do;
the dashboard itself does not start agents or grant approval for their actions.

Subscriptions persist across restarts, default to 24 hours, and grant at most
seven days per refresh. Delivery verifies the callback challenge, signs exact
payload bytes with Standard Webhooks, rejects private callback addresses and
redirects, pins the checked address for TLS, and retries transient failures up to
five times with the same event ID. Verification is cached for one minute per
project/callback/key. Key rotation signs with both keys for five minutes. `410`
deactivates a subscription; `413` and other permanent failures are not retried.
The last 10,000 changes provide a bounded delivery buffer; protocol replay is not
advertised (`cursor: null`). Feedback remains durable and readable after a missed
notification. Review delivery failures in the QR section.

To revoke all subscriptions for a project, use:

```powershell
rdsh-dashboard revoke-events --project C:\Projects\MyProject
```

This is a private, single-owner project credential model, without OAuth or
multi-user roles. Removing a ChatGPT connection must also revoke its subscriptions
or stop the dashboard. Full Dot subscription/trigger validation requires a real
ChatGPT plugin connection; local protocol tests alone do not establish that.

### Connect a private server using Secure MCP Tunnel

Tailscale provides the phone/browser connection. A cloud Dot also needs an MCP
transport reachable from ChatGPT. The supported private option is OpenAI's
[Secure MCP Tunnel](https://developers.openai.com/api/docs/guides/secure-mcp-tunnels).
Its official client is downloadable from
[openai/tunnel-client releases](https://github.com/openai/tunnel-client/releases/latest).
On Windows, extract the client into `%LOCALAPPDATA%\rdsh\tunnel-client`, or set
`RDSH_TUNNEL_CLIENT` to the executable's path.

1. Create a tunnel in OpenAI Platform, associate it with your ChatGPT workspace,
   and obtain a runtime API key with Tunnels Read + Use.
2. Set `CONTROL_PLANE_API_KEY` privately in the terminal. Keep it out of chat,
   command arguments, and Git.
3. With the project dashboard running, start the authenticated HTTP tunnel:

   ```powershell
   rdsh-dashboard tunnel --project C:\Projects\MyProject --tunnel-id tunnel_YOUR_ID
   ```

   The launcher passes the current project bearer through a child-process
   environment reference, including discovery requests. It uses a loopback-only
   health listener with an ephemeral port, recorded in `tunnel-health.url` in the
   project's private state directory. Check `/readyz` and `/ui` at that URL.
   Restart this command after restarting the dashboard.

4. In ChatGPT Plugins, create a private developer connection and choose Tunnel.
   Discover the tools/events, then authorize your Dot to monitor this project's
   `dashboard.answer.created` event and specify its response.
5. Verify a real question → human answer → signed webhook → Dot response, then
   stop monitoring and check that unsubscribe removes the subscription.

Legacy MCP clients also have stdio/Streamable HTTP tools and
`dashboard://state` / `dashboard://feedback` resource subscriptions. Those
resource notifications are distinct from the native webhook Events integration.
See the [official Events contract](https://developers.openai.com/plugins/build/mcp-events).

## Validation

```sh
cd dashboard
npm ci
npm test
```

Tests cover project isolation, durable feedback, HTTP/stdio compatibility,
resource notifications, native MCP 2.0 Events lifecycle, callback signatures,
retry/restart behavior, invalid callbacks, and private Serve conflicts. CI runs
these on Windows and Linux. Browser/phone access and real Dot triggers require
their respective account configurations and are checked separately.
