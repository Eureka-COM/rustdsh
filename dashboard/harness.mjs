import { spawn, execFile } from "node:child_process";
import { promisify } from "node:util";
import http from "node:http";
import net from "node:net";

export async function startHarness(port, frontPort) {
  // A managed instance keeps the original Harness process token and its browser fence intact.
  if (process.platform !== "win32")
    throw new Error(
      "Harness mode currently requires Windows + WSL; project mode is portable",
    );
  const distro = process.env.RDSH_WSL_DISTRO || "FlashNext";
  const wrapper =
    process.env.RDSH_WSL_HARNESS_BIN || "/root/.local/bin/rdsh-env";
  const arguments_ = [
    "-d",
    distro,
    "--exec",
    "bash",
    "-c",
    'printf "RDSH_MANAGED_PID=%s\\n" "$$"; wrapper="$1"; shift; exec "$wrapper" "$@"',
    "rdsh-dashboard",
    wrapper,
    "dsh",
    "--profile",
    "web",
    "--host",
    "127.0.0.1",
    "--port",
    String(port),
    "--no-open",
    "--trusted-host",
    `127.0.0.1:${frontPort}`,
    `localhost:${frontPort}`,
  ];
  const child = spawn("wsl.exe", arguments_, {
    windowsHide: true,
    stdio: ["ignore", "pipe", "pipe"],
  });
  let output = "";
  let linuxPid = null;
  const stop = async () => {
    if (linuxPid)
      await promisify(execFile)(
        "wsl.exe",
        ["-d", distro, "--exec", "kill", "-TERM", String(linuxPid)],
        { windowsHide: true, timeout: 5000 },
      ).catch(() => {});
    child.kill();
  };
  const ready = new Promise((resolve, reject) => {
    const timeout = setTimeout(
      () =>
        reject(
          new Error(
            "Harness startup timed out; check the existing dsh web profile",
          ),
        ),
      45000,
    );
    const capture = (chunk) => {
      output = (output + chunk.toString()).slice(-32000);
      linuxPid =
        Number(output.match(/RDSH_MANAGED_PID=(\d+)/)?.[1]) || linuxPid;
      const match = output.match(
        /dsh web: (http:\/\/127\.0\.0\.1:\d+\/[^\s]*)/,
      );
      if (match) {
        clearTimeout(timeout);
        resolve(new URL(match[1]));
      }
    };
    child.stdout.on("data", capture);
    child.stderr.on("data", capture);
    child.once("error", (error) => {
      clearTimeout(timeout);
      reject(error);
    });
    child.once("exit", (code) => {
      clearTimeout(timeout);
      reject(
        new Error(
          `Harness exited with code ${code}; port ${port} may already be used`,
        ),
      );
    });
  });
  try {
    return { child, url: await ready, port, stop };
  } catch (error) {
    await stop();
    throw error;
  }
}
export function proxyHarness(req, res, port) {
  // Only authenticated same-origin callers reach this proxy. dsh still checks its own process token.
  const headers = { ...req.headers, host: `127.0.0.1:${port}` };
  if (headers.origin) headers.origin = `http://127.0.0.1:${port}`;
  if (headers.referer) headers.referer = `http://127.0.0.1:${port}/`;
  delete headers["x-forwarded-host"];
  delete headers["x-forwarded-proto"];
  const upstream = http.request(
    { hostname: "127.0.0.1", port, path: req.url, method: req.method, headers },
    (response) => {
      res.writeHead(response.statusCode, response.headers);
      response.pipe(res);
    },
  );
  upstream.on("error", () => {
    if (!res.headersSent) res.writeHead(502);
    res.end("Harness is not available");
  });
  req.pipe(upstream);
}
export function upgradeHarness(req, socket, head, port) {
  const upstream = net.connect(port, "127.0.0.1", () => {
    const headers = { ...req.headers, host: `127.0.0.1:${port}` };
    if (headers.origin) headers.origin = `http://127.0.0.1:${port}`;
    const raw = `${req.method} ${req.url} HTTP/1.1\r\n${Object.entries(headers)
      .map(([key, value]) => `${key}: ${value}`)
      .join("\r\n")}\r\n\r\n`;
    upstream.write(raw);
    if (head.length) upstream.write(head);
    upstream.pipe(socket);
    socket.pipe(upstream);
  });
  upstream.on("error", () => socket.destroy());
  socket.on("error", () => upstream.destroy());
  socket.on("close", () => upstream.destroy());
}
