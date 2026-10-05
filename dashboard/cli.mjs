#!/usr/bin/env node
import { parseArgs } from "node:util";
import fs from "node:fs/promises";
import path from "node:path";
import { spawn } from "node:child_process";
import { identity, stateHome } from "./state.mjs";
import { startDashboard } from "./server.mjs";
import { runStdio } from "./mcp.mjs";

const help = `rdsh-dashboard project --project <directory> [--port <port>] [--no-tailscale] [--open]
rdsh-dashboard harness [--port 38081] [--harness-port 3081] [--no-tailscale] [--open]
rdsh-dashboard open --project <directory> | --harness
rdsh-dashboard stop --project <directory> | --harness
rdsh-dashboard revoke-events --project <directory>
rdsh-dashboard tunnel --project <directory> --tunnel-id <tunnel_id>
rdsh-dashboard mcp --project <directory>

Project mode: project metrics, tasks, questions, human feedback, and /mcp.
Harness mode: a separate managed DeepSeek Harness Web UI and QR landing page.
Tailscale Serve shares each loopback server privately over HTTPS.
`;
const { values, positionals } = parseArgs({
  allowPositionals: true,
  options: {
    project: { type: "string" },
    port: { type: "string" },
    "harness-port": { type: "string" },
    "no-tailscale": { type: "boolean" },
    "tunnel-id": { type: "string" },
    open: { type: "boolean" },
    harness: { type: "boolean" },
    help: { type: "boolean", short: "h" },
  },
});
function openUrl(url) {
  if (process.platform !== "win32")
    throw new Error("Browser opening is supported by the Windows launcher");
  const child = spawn("rundll32.exe", ["url.dll,FileProtocolHandler", url], {
    windowsHide: true,
    detached: true,
    stdio: "ignore",
  });
  child.unref();
}
function portValue(value) {
  if (value === undefined) return undefined;
  const port = Number(value);
  if (!Number.isInteger(port) || port < 1024 || port > 65535)
    throw new Error("Port must be an integer between 1024 and 65535");
  return port;
}
try {
  const command = positionals[0];
  if (values.help || !command) {
    console.log(help);
  } else if (["open", "stop", "revoke-events"].includes(command)) {
    const project = values.harness
      ? null
      : await identity(values.project || process.cwd());
    const directory = project
      ? project.directory
      : path.join(stateHome(), "harness");
    const runtime = JSON.parse(
      await fs.readFile(path.join(directory, "runtime.json"), "utf8"),
    );
    try {
      process.kill(runtime.pid, 0);
    } catch {
      throw new Error("Dashboard is stopped; start it first");
    }
    if (command === "open") openUrl(runtime.browser_url);
    else {
      if (command === "revoke-events" && values.harness)
        throw new Error("Events are available in project mode only");
      const prefix = values.harness ? "_rdsh/" : "";
      const response = await fetch(
        runtime.local_url +
          prefix +
          "api/" +
          (command === "stop" ? "stop" : "events/revoke"),
        {
          method: "POST",
          headers: { authorization: `Bearer ${runtime.token}` },
          signal: AbortSignal.timeout(15000),
        },
      );
      if (!response.ok)
        throw new Error(
          (await response.json()).error || `HTTP ${response.status}`,
        );
      console.log(
        `[rdsh-dashboard] ${command === "stop" ? "Stopping dashboard" : "Event subscriptions revoked"}`,
      );
    }
  } else if (command === "tunnel") {
    if (!/^tunnel_[a-z0-9]{32}$/.test(values["tunnel-id"] || ""))
      throw new Error(
        "Provide the tunnel_id from OpenAI Platform tunnel settings",
      );
    if (!process.env.CONTROL_PLANE_API_KEY)
      throw new Error(
        "Set CONTROL_PLANE_API_KEY locally before connecting; do not put keys in command arguments or chat",
      );
    const project = await identity(values.project || process.cwd());
    const runtime = JSON.parse(
      await fs.readFile(path.join(project.directory, "runtime.json"), "utf8"),
    );
    const response = await fetch(runtime.local_url + "api/state", {
      headers: { authorization: `Bearer ${runtime.token}` },
      signal: AbortSignal.timeout(5000),
    });
    if (!response.ok)
      throw new Error(
        "Start the project dashboard before connecting its tunnel",
      );
    const command =
      process.env.RDSH_TUNNEL_CLIENT ||
      (process.platform === "win32"
        ? path.join(
            process.env.LOCALAPPDATA,
            "rdsh",
            "tunnel-client",
            "tunnel-client.exe",
          )
        : "tunnel-client");
    const child = spawn(
      command,
      [
        "run",
        "--control-plane.tunnel-id",
        values["tunnel-id"],
        "--control-plane.api-key",
        "env:CONTROL_PLANE_API_KEY",
        "--mcp.server-url",
        runtime.local_url + "mcp",
        "--mcp.extra-headers",
        "Authorization: env:RDSH_DASHBOARD_AUTHORIZATION",
        "--mcp.discovery-extra-headers",
        "Authorization: env:RDSH_DASHBOARD_AUTHORIZATION",
        "--health.listen-addr",
        "127.0.0.1:0",
        "--health.url-file",
        path.join(project.directory, "tunnel-health.url"),
      ],
      {
        windowsHide: true,
        stdio: "inherit",
        env: {
          ...process.env,
          RDSH_DASHBOARD_AUTHORIZATION: `Bearer ${runtime.mcp_token}`,
        },
      },
    );
    process.exitCode = await new Promise((resolve, reject) => {
      child.once("error", reject);
      child.once("exit", (code) => resolve(code ?? 1));
    });
  } else if (command === "mcp") {
    await runStdio(await identity(values.project || process.cwd()));
  } else if (command === "project" || command === "harness") {
    const dashboard = await startDashboard({
      kind: command,
      project:
        command === "project"
          ? await identity(values.project || process.cwd())
          : null,
      port: portValue(values.port),
      harnessPort: portValue(values["harness-port"]),
      tailscale: !values["no-tailscale"],
    });
    console.log(`[rdsh-dashboard] ${command}: ${dashboard.localUrl}`);
    console.log(`[rdsh-dashboard] Tailscale: ${dashboard.getShare().state}`);
    console.log(`[rdsh-dashboard] ${dashboard.getShare().message}`);
    console.log(
      "[rdsh-dashboard] Open with rdsh-dashboard open; Ctrl-C to stop.",
    );
    if (values.open) openUrl(dashboard.browserUrl);
    for (const signal of ["SIGINT", "SIGTERM"])
      process.once(signal, async () => {
        await dashboard.close();
        process.exit(0);
      });
  } else throw new Error("Unknown command; use --help");
} catch (e) {
  console.error(`[rdsh-dashboard] ${e.message}`);
  process.exitCode = 1;
}
