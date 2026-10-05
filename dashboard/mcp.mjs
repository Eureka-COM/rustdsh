import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
  ListResourcesRequestSchema,
  ReadResourceRequestSchema,
  SubscribeRequestSchema,
  UnsubscribeRequestSchema,
} from "@modelcontextprotocol/sdk/types.js";
import fs from "node:fs/promises";
import path from "node:path";
import { metricNames } from "./state.mjs";

const string = { type: "string", minLength: 1, maxLength: 8000 };
const object = (properties, required = []) => ({
  type: "object",
  properties,
  required,
  additionalProperties: false,
});
export const tools = [
  {
    name: "dashboard_update_metrics",
    description:
      "Replace reported project metrics with measured values. Omit unknown metrics; never estimate costs or cache/error rates from unrelated data. Counters are cumulative snapshots, not increments.",
    inputSchema: object(
      Object.fromEntries([
        ...metricNames.map((name) => [
          name,
          { type: ["number", "null"], minimum: 0 },
        ]),
        ["session_id", string],
      ]),
    ),
  },
  {
    name: "dashboard_upsert_task",
    description:
      "Create or update a project task shown in the project dashboard.",
    inputSchema: object(
      {
        id: string,
        title: string,
        status: { enum: ["todo", "doing", "done", "blocked"] },
        milestone: string,
        blocker: string,
      },
      ["id", "title", "status"],
    ),
  },
  {
    name: "dashboard_ask_question",
    description:
      "Place a question in the project dashboard for a human to answer. A default action is informational; this server never executes it automatically. Retrieve replies with dashboard_get_feedback or subscribe to dashboard://feedback.",
    inputSchema: object(
      {
        id: string,
        question: string,
        urgency: { enum: ["normal", "high", "critical"] },
        default_action: string,
      },
      ["id", "question"],
    ),
  },
  {
    name: "dashboard_publish_event",
    description:
      "Publish progress, a note, or an artifact reference. Local files are referenced by name/path only; this server does not read or publish their contents.",
    inputSchema: object(
      {
        type: { enum: ["progress", "artifact", "note"] },
        title: string,
        detail: string,
        artifact: string,
      },
      ["title"],
    ),
  },
  {
    name: "dashboard_get_feedback",
    description:
      "Read durable human answers after a sequence cursor. Pass next_cursor on the next call. Reads never consume or delete feedback; poll at appropriate workflow checkpoints if resource subscriptions are unavailable.",
    inputSchema: object({ after: { type: "integer", minimum: 0 } }),
  },
  {
    name: "dashboard_get_state",
    description:
      "Read this project’s metrics, tasks, questions, and recent events.",
    inputSchema: object({}),
  },
  {
    name: "dashboard_check_task_contract",
    description:
      "Check declared repository, cwd and write paths against the current task contract before an operation. Only within_scope passes this preflight; block and unparsed must stop. This does not grant approval, parse commands, or enforce an OS sandbox. Recheck immediately before use; contracts can change.",
    inputSchema: object({
      task_id: string,
      contract_version: { type: "integer", minimum: 1 },
      repository: string,
      cwd: string,
      write_paths: { type: "array", maxItems: 100, items: string },
    }, ["task_id", "contract_version", "repository", "cwd", "write_paths"]),
  },
];
const routes = {
  dashboard_update_metrics: "metrics",
  dashboard_upsert_task: "task",
  dashboard_ask_question: "question",
  dashboard_publish_event: "event",
};
export async function executeTool(api, name, args = {}) {
  if (name === "dashboard_get_state") return await api.getState();
  if (name === "dashboard_check_task_contract") return await api.checkContract(args);
  if (name === "dashboard_get_feedback")
    return feedbackSince(await api.getState(), args.after ?? 0);
  if (routes[name]) {
    const state = await api.mutate(routes[name], args);
    return { project: state.project.id, revision: state.revision };
  }
  throw new Error("Unknown tool");
}
const resourceDefinitions = [
  {
    uri: "dashboard://state",
    name: "Project dashboard state",
    mimeType: "application/json",
  },
  {
    uri: "dashboard://feedback",
    name: "Human answers",
    mimeType: "application/json",
  },
];
export function feedbackSince(state, after = 0) {
  if (!Number.isSafeInteger(after) || after < 0)
    throw new Error("after must be a non-negative integer");
  const messages = state.feedback
    .filter((item) => item.sequence > after)
    .slice(0, 100);
  return { messages, next_cursor: messages.at(-1)?.sequence ?? after };
}
export function createMcpServer(api) {
  const server = new Server(
    { name: "rdsh-project-dashboard", version: "0.1.0" },
    { capabilities: { tools: {}, resources: { subscribe: true } } },
  );
  const subscriptions = new Set();
  server.setRequestHandler(ListToolsRequestSchema, async () => ({ tools }));
  server.setRequestHandler(CallToolRequestSchema, async (request) => {
    try {
      const args = request.params.arguments || {};
      const name = request.params.name;
      const result = await executeTool(api, name, args);
      return { content: [{ type: "text", text: JSON.stringify(result) }] };
    } catch (error) {
      return {
        isError: true,
        content: [{ type: "text", text: error.message }],
      };
    }
  });
  server.setRequestHandler(ListResourcesRequestSchema, async () => ({
    resources: resourceDefinitions,
  }));
  server.setRequestHandler(ReadResourceRequestSchema, async (request) => {
    const uri = request.params.uri;
    if (!resourceDefinitions.some((resource) => resource.uri === uri))
      throw new Error("Unknown resource");
    const state = await api.getState();
    return {
      contents: [
        {
          uri,
          mimeType: "application/json",
          text: JSON.stringify(
            uri === "dashboard://feedback" ? state.feedback : state,
          ),
        },
      ],
    };
  });
  server.setRequestHandler(SubscribeRequestSchema, async (request) => {
    if (
      !resourceDefinitions.some(
        (resource) => resource.uri === request.params.uri,
      )
    )
      throw new Error("Unknown resource");
    subscriptions.add(request.params.uri);
    return {};
  });
  server.setRequestHandler(UnsubscribeRequestSchema, async (request) => {
    subscriptions.delete(request.params.uri);
    return {};
  });
  return {
    server,
    notify: async () => {
      for (const uri of subscriptions)
        await server.sendResourceUpdated({ uri }).catch(() => {});
    },
  };
}
export async function runStdio(project) {
  const readRuntime = async () =>
    JSON.parse(
      await fs.readFile(path.join(project.directory, "runtime.json"), "utf8"),
    );
  async function request(route, input) {
    const runtime = await readRuntime();
    if (runtime.kind !== "project" || runtime.project_id !== project.id)
      throw new Error("Dashboard runtime belongs to a different project");
    const response = await fetch(`${runtime.local_url}${route}`, {
      method: input ? "POST" : "GET",
      headers: {
        authorization: `Bearer ${runtime.mcp_token}`,
        "content-type": "application/json",
      },
      body: input ? JSON.stringify(input) : undefined,
      signal: AbortSignal.timeout(10000),
    });
    const result = await response.json();
    if (!response.ok && !(route === "api/contracts/check" && response.status === 409))
      throw new Error(result.error || `HTTP ${response.status}`);
    return result;
  }
  const mcp = createMcpServer({
    getState: () => request("api/state"),
    mutate: (operation, input) => request(`api/update/${operation}`, input),
    checkContract: (input) => request("api/contracts/check", input),
  });
  await mcp.server.connect(new StdioServerTransport());
  // Resource subscribers receive change notifications; disconnected clients can recover with cursors.
  let revision = null;
  const timer = setInterval(async () => {
    try {
      const state = await request("api/state");
      if (revision !== null && state.revision !== revision) await mcp.notify();
      revision = state.revision;
    } catch {}
  }, 2000);
  timer.unref();
  mcp.server.onclose = () => clearInterval(timer);
}
