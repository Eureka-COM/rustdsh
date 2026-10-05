import { createHash, randomBytes } from "node:crypto";
import fs from "node:fs/promises";
import path from "node:path";
import os from "node:os";
import { prepareContract } from "./contracts.mjs";
import { evaluateOperation } from "./policy.mjs";
import { prepareApprovalRequest, decideApproval, claimApproval, checkApproval } from "./approvals.mjs";

export const metricNames = [
  "total_cost_usd",
  "total_budget_usd",
  "session_cost_usd",
  "session_budget_usd",
  "input_tokens",
  "cached_input_tokens",
  "model_calls",
  "tool_calls",
  "tool_errors",
  "context_misses",
  "auto_continues",
  "refusals",
  "api_errors",
];
export function stateHome() {
  return (
    process.env.RDSH_DASHBOARD_HOME ||
    path.join(
      process.env.LOCALAPPDATA || path.join(os.homedir(), ".local", "state"),
      "rdsh",
      "dashboard",
    )
  );
}
export async function identity(project) {
  const root = await fs.realpath(path.resolve(project));
  if (!(await fs.stat(root)).isDirectory())
    throw new Error("Project must be a directory");
  const normalized = process.platform === "win32" ? root.toLowerCase() : root;
  const id = createHash("sha256").update(normalized).digest("hex").slice(0, 16);
  return {
    id,
    root,
    name: path.basename(root),
    directory: path.join(stateHome(), "projects", id),
  };
}
export async function writeJson(file, value) {
  await fs.mkdir(path.dirname(file), { recursive: true });
  const temp = `${file}.${process.pid}.${randomBytes(4).toString("hex")}.tmp`;
  await fs.writeFile(temp, JSON.stringify(value, null, 2) + "\n", {
    mode: 0o600,
  });
  await fs.rename(temp, file);
}
export class ProjectStore {
  constructor(project, value) {
    this.project = project;
    this.value = value;
  }
  static async open(project) {
    let value;
    try {
      value = JSON.parse(
        await fs.readFile(path.join(project.directory, "state.json"), "utf8"),
      );
    } catch (e) {
      if (e.code !== "ENOENT") throw e;
      value = {
        schema: 1,
        project: { id: project.id, name: project.name, root: project.root },
        revision: 0,
        updated_at: null,
        metrics: {},
        tasks: [],
        contracts: [],
        questions: [],
        events: [],
        feedback: [],
      };
    }
    if (value.project.id !== project.id || value.schema !== 1)
      throw new Error(
        "Dashboard state belongs to a different project or version",
      );
    return new ProjectStore(project, value);
  }
  async mutate(operation, input, authority = "agent") {
    const next = structuredClone(this.value);
    if (operation === "contract") {
      if (authority !== "local_administrator")
        throw new Error("Only the local administrator can change a task contract");
      const version = await prepareContract(next, input);
      next.contracts ||= [];
      let history = next.contracts.find((item) => item.task_id === input.task_id);
      if (!history) {
        history = { task_id: input.task_id, versions: [] };
        next.contracts.push(history);
      }
      history.versions.push(version);
    } else if (operation === "approval_request") {
      const version = await prepareApprovalRequest(next, input);
      next.approval_requests ||= [];
      let history = next.approval_requests.find((item) => item.id === input.id);
      if (!history) {
        history = { id: input.id, versions: [] };
        next.approval_requests.push(history);
      }
      history.versions.push(version);
    } else if (operation === "approval_decision") {
      decideApproval(next, input, authority);
    } else if (["approval_claim", "approval_check"].includes(operation)) {
      const result = operation === "approval_claim" ? await claimApproval(next, input) : await checkApproval(next, input);
      next.approval_checks ||= [];
      next.approval_checks.push({ checked_at: new Date().toISOString(), ...result });
      next.approval_checks = next.approval_checks.slice(-1000);
    } else if (operation === "policy") {
      const result = await evaluateOperation(next, input);
      next.policy_checks ||= [];
      next.policy_checks.push({
        id: `policy_${next.revision + 1}`, checked_at: new Date().toISOString(), ...result,
      });
      next.policy_checks = next.policy_checks.slice(-1000);
    } else {
      applyOperation(next, operation, input);
    }
    next.revision++;
    next.updated_at = new Date().toISOString();
    const names = {
      answer: "dashboard.answer.created",
      question: "dashboard.question.created",
      task: "dashboard.task.updated",
      event: "dashboard.progress.updated",
      metrics: "dashboard.metrics.updated",
      contract: "dashboard.contract.updated",
      policy: "dashboard.policy.checked",
      approval_request: "dashboard.approval.requested",
      approval_decision: "dashboard.approval.decided",
      approval_claim: "dashboard.approval.claimed",
      approval_check: "dashboard.approval.checked",
    };
    const summary =
      operation === "answer"
        ? input.answer
        : operation === "question"
          ? input.question
          : operation === "contract"
            ? "タスク契約を更新"
            : operation === "policy"
              ? "操作構造を照合"
              : operation.startsWith("approval_")
                ? "操作承認の台帳を更新"
                : input.title || "指標を更新";
    next.changes ||= [];
    next.changes.push({
      eventId: `evt_${this.project.id}_${next.revision}`,
      name: names[operation],
      timestamp: next.updated_at,
      data: {
        project_id: this.project.id,
        revision: next.revision,
        entity_id: input?.id || input?.task_id || "",
        summary: String(summary).slice(0, 1000),
      },
      cursor: null,
    });
    next.changes = next.changes.slice(-10000);
    await writeJson(path.join(this.project.directory, "state.json"), next);
    this.value = next;
    return next;
  }
}
export function publicState(value) {
  const { changes, ...visible } = value;
  return visible;
}
function text(value, label, max = 8000) {
  if (typeof value !== "string" || !value.trim() || value.length > max)
    throw new Error(
      `${label} must be a non-empty string (max ${max} characters)`,
    );
  return value;
}
function oneOf(value, values, label) {
  if (!values.includes(value)) throw new Error(`Invalid ${label}`);
  return value;
}
export function applyOperation(state, operation, input) {
  if (!input || typeof input !== "object" || Array.isArray(input))
    throw new Error("Expected an object");
  switch (operation) {
    case "metrics": {
      if (
        Object.keys(input).some(
          (key) => !metricNames.includes(key) && key !== "session_id",
        )
      )
        throw new Error("Unknown metric");
      for (const key of metricNames) {
        if (!(key in input)) continue;
        const n = input[key];
        if (
          n !== null &&
          (typeof n !== "number" ||
            !Number.isFinite(n) ||
            n < 0 ||
            (!key.endsWith("_usd") && !Number.isInteger(n)))
        )
          throw new Error(`Invalid ${key}`);
        state.metrics[key] = n;
      }
      if ("session_id" in input)
        state.metrics.session_id = text(input.session_id, "session_id", 160);
      const m = state.metrics;
      if (
        m.cached_input_tokens != null &&
        m.input_tokens != null &&
        m.cached_input_tokens > m.input_tokens
      )
        throw new Error("Cached tokens exceed input tokens");
      if (
        m.tool_errors != null &&
        m.tool_calls != null &&
        m.tool_errors > m.tool_calls
      )
        throw new Error("Tool errors exceed tool calls");
      break;
    }
    case "task": {
      const task = {
        id: text(input.id, "id", 160),
        title: text(input.title, "title", 1000),
        status: oneOf(
          input.status,
          ["todo", "doing", "done", "blocked"],
          "task status",
        ),
        milestone: input.milestone
          ? text(input.milestone, "milestone", 160)
          : "",
        blocker: input.blocker ? text(input.blocker, "blocker", 2000) : "",
        updated_at: new Date().toISOString(),
      };
      const index = state.tasks.findIndex((item) => item.id === task.id);
      if (index < 0) state.tasks.push(task);
      else state.tasks[index] = task;
      break;
    }
    case "question": {
      const question = {
        id: text(input.id, "id", 160),
        question: text(input.question, "question"),
        urgency: oneOf(
          input.urgency || "normal",
          ["normal", "high", "critical"],
          "urgency",
        ),
        default_action: input.default_action
          ? text(input.default_action, "default_action", 2000)
          : "",
        created_at: new Date().toISOString(),
        answer: null,
      };
      if (state.questions.some((item) => item.id === question.id))
        throw new Error(
          "Question id already exists; use a new id for a follow-up",
        );
      state.questions.push(question);
      break;
    }
    case "answer": {
      const question = state.questions.find((item) => item.id === input.id);
      if (!question) throw new Error("Question not found");
      if (question.answer !== null)
        throw new Error("Question already answered");
      question.answer = text(input.answer, "answer");
      question.answered_at = new Date().toISOString();
      state.feedback.push({
        sequence: (state.feedback.at(-1)?.sequence || 0) + 1,
        type: "question_answered",
        question_id: question.id,
        question: question.question,
        answer: question.answer,
        created_at: question.answered_at,
      });
      break;
    }
    case "event": {
      const event = {
        sequence: (state.events.at(-1)?.sequence || 0) + 1,
        type: oneOf(
          input.type || "progress",
          ["progress", "artifact", "note"],
          "event type",
        ),
        title: text(input.title, "title", 1000),
        detail: input.detail ? text(input.detail, "detail") : "",
        artifact: input.artifact ? text(input.artifact, "artifact", 2000) : "",
        created_at: new Date().toISOString(),
      };
      state.events.push(event);
      // Full state and all feedback are durable; retain the latest 1000 display events.
      state.events = state.events.slice(-1000);
      break;
    }
    default:
      throw new Error("Unknown operation");
  }
}
