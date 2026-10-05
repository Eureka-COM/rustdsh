import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs/promises";
import vm from "node:vm";

test("task rows display current and historical contracts as literal text", async () => {
  class Element {
    constructor(tag) {
      this.tag = tag;
      this.children = [];
      this.dataset = {};
      this.style = {};
      this.textContent = "";
    }
    set innerHTML(_value) { throw new Error("Contract content must use textContent"); }
    append(...children) { this.children.push(...children); }
    replaceChildren(...children) { this.children = children; }
    setAttribute() {}
    addEventListener() {}
    querySelectorAll() { return []; }
  }
  const elements = new Map();
  const document = {
    createElement: (tag) => new Element(tag),
    getElementById: (id) => {
      if (!elements.has(id)) elements.set(id, new Element("div"));
      return elements.get(id);
    },
  };
  const unsafe = '<img src=x onerror="forged approval">';
  const version = {
    version: 1, changed_at: "2026-10-05T00:00:00Z", purpose: unsafe,
    repository: "/fixture", allowed_scope: "Source only", write_roots: ["/fixture/src"],
    forbidden_actions: ["No publication"], completion_conditions: ["Tests pass"],
    change_reason: "Initial contract",
  };
  const state = {
    metrics: {}, tasks: [{ id: "T1", title: "Test", status: "todo", blocker: "" }, { id: "T2", title: "Legacy task", status: "todo", blocker: "" }],
    questions: [], events: [], updated_at: null,
    contracts: [{ task_id: "T1", versions: [version, { ...version, version: 2, write_roots: [], change_reason: "Read only" }] }],
  };
  const source = await fs.readFile(new URL("../app.mjs", import.meta.url), "utf8");
  await vm.runInNewContext(`(async () => {\n${source}\n})()`, {
    document, location: { pathname: "/", search: "", hash: "" },
    sessionStorage: { getItem: () => null, setItem() {} }, history: { replaceState() {} },
    URL, URLSearchParams,
    EventSource: class { addEventListener() {} },
    fetch: async (route) => ({
      ok: true,
      json: async () => route.endsWith("config") ? {
        kind: "project", project: { name: "Test", root: "/fixture" },
        share: { state: "disabled", message: "Local fixture", url: null },
      } : state,
    }),
  });
  const descendants = (element) => [element, ...element.children.flatMap(descendants)];
  const nodes = descendants(elements.get("tasks"));
  const labels = nodes.map((node) => node.textContent);
  assert.ok(labels.includes("タスク契約 v2"));
  assert.ok(labels.includes("タスク契約 未設定"));
  assert.ok(labels.some((label) => label.startsWith("v1 ・")));
  assert.ok(labels.some((label) => label.startsWith("v2 ・")));
  assert.ok(labels.includes("目的: " + unsafe));
  assert.ok(labels.includes("書込先: 書込不可"));
  assert.ok(labels.includes("終了条件: Tests pass"));
  assert.ok(labels.includes("変更理由: Read only"));
  assert.ok(nodes.every((node) => node.tag !== "img" && node.tag !== "script"));
});
