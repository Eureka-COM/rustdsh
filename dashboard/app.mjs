import { metricView, observationView, ratioView } from "./observations.mjs";

const $ = (id) => document.getElementById(id);
const base = location.pathname.startsWith("/_rdsh") ? "/_rdsh/" : "/";
const browserToken = base === "/"
  ? new URLSearchParams(location.hash.slice(1)).get("key") || sessionStorage.getItem("rdsh_project_browser_token") || ""
  : "";
if (browserToken) {
  sessionStorage.setItem("rdsh_project_browser_token", browserToken);
  history.replaceState(null, "", location.pathname + location.search);
}
async function api(route, body) {
  const headers = browserToken ? { "x-rdsh-browser-token": browserToken } : {};
  const response = await fetch(
    base + "api/" + route,
    body === undefined
      ? { headers }
      : {
          method: "POST",
          headers: { ...headers, "content-type": "application/json" },
          body: JSON.stringify(body),
        },
  );
  const result = await response.json();
  if (!response.ok) throw new Error(result.error || "接続できません");
  return result;
}
function node(tag, text, className) {
  const result = document.createElement(tag);
  if (text !== undefined) result.textContent = text;
  if (className) result.className = className;
  return result;
}
const number = (value) =>
  value == null ? "未取得" : value.toLocaleString("ja-JP");
const money = (value) => (value == null ? "未取得" : "$" + value.toFixed(2));
const ratio = (numerator, denominator) =>
  numerator == null || denominator == null || denominator === 0
    ? null
    : numerator / denominator;
const percentage = (value) =>
  value == null ? "未取得" : (value * 100).toFixed(1) + "%";
function card(label, value, detail, progress, warning = false) {
  const element = node("section", undefined, "card");
  element.append(
    node("div", label, "label"),
    node("div", value, "value" + (warning ? " warn" : "")),
    node("div", detail, "detail"),
  );
  if (progress != null) {
    const bar = node("div", undefined, "bar");
    const fill = node("span");
    fill.style.width = Math.min(100, Math.max(0, progress * 100)) + "%";
    bar.append(fill);
    element.append(bar);
  }
  return element;
}
function emptyRow(text, columns) {
  const tr = node("tr");
  const cell = node("td", text, "empty");
  cell.colSpan = columns;
  tr.append(cell);
  return tr;
}
const date = (value) =>
  value ? new Date(value).toLocaleString("ja-JP") : "未申告";
const freshnessLabels = {
  fresh: "",
  stale: "古い情報",
  unknown: "鮮度未確認",
  unavailable: "未取得",
};
let openObservations = new Set();
function provenance(view, label, format = String, id = "") {
  const element = node("div", undefined, "observation");
  element.dataset.freshness = view.freshness;
  element.dataset.kind = view.kind || "unknown";
  const heading = node("div", undefined, "observation-heading");
  if (label) heading.append(node("span", label + "："));
  heading.append(
    node("span", view.label, "provenance-badge " + (view.kind || "unknown")),
  );
  if (view.freshness === "stale" || view.freshness === "unknown")
    heading.append(
      node("span", freshnessLabels[view.freshness], "provenance-badge " + view.freshness),
    );
  element.append(heading);
  if (view.value != null && !view.current)
    element.append(node("div", "前回の報告値：" + format(view.value), "sub"));
  const o = view.observation;
  element.append(node("div", "観測：" + date(o?.observed_at), "sub"));
  const details = node("details", undefined, "observation-details");
  details.dataset.observationId = id;
  details.open = openObservations.has(id);
  const summary = node("summary", "報告元・参照対象");
  summary.dataset.observationId = id;
  details.append(
    summary,
    node("div", "報告元：" + (o?.source || "未申告")),
    node("div", "報告session：" + (o?.session_id || "未申告")),
    node("div", "参照対象：" + (o?.reference || "未申告")),
    node("div", "受信：" + date(o?.recorded_at)),
  );
  if (o?.observed_at && o.max_age_seconds) {
    const expiresAt = new Date(Date.parse(o.observed_at) + o.max_age_seconds * 1000);
    details.append(node("div", "鮮度期限：" + date(expiresAt.toISOString())));
  }
  element.append(details);
  return element;
}
function metricText(state, key, format, now) {
  const view = metricView(state, key, now);
  return view.current
    ? (view.kind === "estimated" ? "推定 " : "") + format(view.value)
    : freshnessLabels[view.freshness];
}
function metricCard(state, now, label, value, detail, fields, progress, warning) {
  const element = card(label, value, detail, progress, warning);
  for (const [name, key, format = number] of fields)
    element.append(
      provenance(metricView(state, key, now), name, format, "metric:" + key),
    );
  return element;
}
function rateText(state, a, b, now) {
  const result = ratioView(state, a, b, now);
  if (result.reason === "incompatible") return "比較不可";
  if (result.value !== null)
    return (metricView(state, a, now).kind === "estimated" ? "推定 " : "") + percentage(result.value);
  const views = [metricView(state, a, now), metricView(state, b, now)];
  if (views.some((view) => view.freshness === "stale")) return "古い情報";
  if (views.some((view) => view.freshness === "unknown")) return "鮮度未確認";
  return "未取得";
}
function freshnessSignature(state, now) {
  return JSON.stringify([
    Object.keys(state.metrics).map((key) => metricView(state, key, now).freshness),
    state.tasks.map((task) => observationView(task.status, task.observation, now).freshness),
    state.events.slice(-30).map((event) => observationView(event.title, event.observation, now).freshness),
  ]);
}
let reportFreshnessSignature = null;
function renderReports(state, now = Date.now()) {
  openObservations = new Set(
    [...document.querySelectorAll("details.observation-details[open]")]
      .map((element) => element.dataset.observationId),
  );
  const activeDisclosure = document.activeElement?.dataset?.observationId;
  const metric = (key, format = number) => metricText(state, key, format, now);
  const progress = (a, b) => ratio(
    metricView(state, a, now).current ? state.metrics[a] : null,
    metricView(state, b, now).current ? state.metrics[b] : null,
  );
  const currentDone = state.tasks.filter((task) => {
    const view = observationView(task.status, task.observation, now);
    return task.status === "done" && view.current && view.kind !== "estimated";
  }).length;
  const pending = state.questions.filter((question) => question.answer === null);
  const counts = ["done", "doing", "todo", "blocked"]
    .map(
      (status) =>
        `${status} ${state.tasks.filter((task) => task.status === status).length}`,
    )
    .join(" · ");
  $("cards").replaceChildren(
    metricCard(state, now,
      "費用（API換算、累計）",
      metric("total_cost_usd", money),
      "上限 " + metric("total_budget_usd", money),
      [["費用", "total_cost_usd", money], ["上限", "total_budget_usd", money]],
      progress("total_cost_usd", "total_budget_usd"),
    ),
    metricCard(state, now,
      "直近のセッション",
      metric("session_cost_usd", money),
      `${state.metrics.session_id || "session未申告"}　上限 ${metric("session_budget_usd", money)}`,
      [["費用", "session_cost_usd", money], ["上限", "session_budget_usd", money]],
      progress("session_cost_usd", "session_budget_usd"),
    ),
    metricCard(state, now,
      "キャッシュ読み込み率",
      rateText(state, "cached_input_tokens", "input_tokens", now),
      `${metric("model_calls")} 回の呼び出し（入力トークン加重）`,
      [["キャッシュ", "cached_input_tokens"], ["入力", "input_tokens"], ["呼び出し", "model_calls"]],
    ),
    metricCard(state, now,
      "ツールのエラー率",
      rateText(state, "tool_errors", "tool_calls", now),
      `${metric("tool_errors")} / ${metric("tool_calls")} 件`,
      [["エラー", "tool_errors"], ["ツール", "tool_calls"]],
    ),
    metricCard(state, now,
      "文脈の読み落とし",
      metric("context_misses"),
      "報告元で検出した回数",
      [["検出数", "context_misses"]],
      undefined,
      metricView(state, "context_misses", now).current && state.metrics.context_misses > 0,
    ),
    metricCard(state, now,
      "自動続行",
      metric("auto_continues"),
      `拒否 ${metric("refusals")} · APIエラー ${metric("api_errors")}`,
      [["続行", "auto_continues"], ["拒否", "refusals"], ["APIエラー", "api_errors"]],
    ),
    card("鮮度内の完了報告", `${currentDone} / ${state.tasks.length}`, "全報告の内訳：" + counts),
    card("未回答の質問", String(pending.length), `回答済み ${state.questions.length - pending.length}`),
  );
  $("task-milestones").textContent = [
    ...new Set(state.tasks.map((task) => task.milestone).filter(Boolean)),
  ].join(" / ");
  $("tasks").replaceChildren(
    ...state.tasks.map((task) => {
      const tr = node("tr");
      const status = node("td");
      const view = observationView(task.status, task.observation, now);
      const statusLabel = (view.kind === "estimated" ? "推定 " : "") +
        task.status + (view.current ? "" : "（" + freshnessLabels[view.freshness] + "）");
      status.append(node(
        "span", statusLabel,
        "status " + (view.current && view.kind !== "estimated" ? task.status : ""),
      ));
      const title = node("td", task.title);
      title.append(provenance(view, undefined, String, "task:" + task.id));
      tr.append(
        node("td", task.id, "id"),
        status,
        title,
        node("td", task.blocker),
      );
      return tr;
    }),
  );
  if (!state.tasks.length)
    $("tasks").append(emptyRow("タスクはまだ登録されていません", 4));
  renderEvents(state, now);
  reportFreshnessSignature = freshnessSignature(state, now);
  if (activeDisclosure)
    [...document.querySelectorAll("summary[data-observation-id]")]
      .find((summary) => summary.dataset.observationId === activeDisclosure)?.focus();
}
let lastState = null;
function render(state) {
  lastState = state;
  renderReports(state);
  const pending = state.questions.filter((question) => question.answer === null);
  // Preserve in-progress human drafts while incoming events refresh the dashboard.
  const drafts = new Map(
    [...$("questions").querySelectorAll("textarea")].map((area) => [
      area.dataset.id,
      area.value,
    ]),
  );
  const active = document.activeElement?.dataset?.id;
  $("questions").replaceChildren(
    ...pending.map((question) => {
      const tr = node("tr"),
        cell = node("td", undefined, "question");
      cell.append(node("div", question.question));
      const form = node("form"),
        textarea = node("textarea");
      textarea.dataset.id = question.id;
      textarea.setAttribute("aria-label", `質問 ${question.id} への回答`);
      textarea.required = true;
      textarea.maxLength = 8000;
      textarea.value = drafts.get(question.id) || "";
      const button = node("button", "回答を返す", "primary");
      button.type = "submit";
      const error = node("div", undefined, "error");
      error.setAttribute("role", "alert");
      form.append(textarea, button, error);
      form.addEventListener("submit", async (event) => {
        event.preventDefault();
        button.disabled = true;
        try {
          await api("update/answer", {
            id: question.id,
            answer: textarea.value,
          });
          await refreshState();
        } catch (e) {
          error.textContent = e.message;
          button.disabled = false;
        }
      });
      cell.append(form);
      tr.append(
        node("td", question.id, "id"),
        node("td", question.urgency),
        cell,
        node("td", question.default_action || "指定なし"),
      );
      return tr;
    }),
  );
  if (!pending.length) $("questions").append(emptyRow("なし", 4));
  if (active)
    [...$("questions").querySelectorAll("textarea")]
      .find((area) => area.dataset.id === active)
      ?.focus();
  $("answers").replaceChildren(
    ...state.questions
      .filter((question) => question.answer !== null)
      .slice()
      .reverse()
      .map((question) => {
        const element = node("article", undefined, "event");
        element.append(
          node("strong", question.question),
          node("p", question.answer),
        );
        return element;
      }),
  );
  $("connection").textContent = "接続済み · プロジェクト専用";
  $("updated").textContent =
    `最終受信: ${state.updated_at ? date(state.updated_at) : "まだ報告がありません"} · 鮮度は各項目の観測時刻から判定します。費用は報告元のAPI換算値です。`;
}
function renderEvents(state, now) {
  $("events").replaceChildren(
    ...state.events
      .slice(-30)
      .reverse()
      .map((event) => {
        const element = node("article", undefined, "event");
        element.append(
          node("strong", event.title),
          node(
            "div",
            `${event.type} · ${new Date(event.created_at).toLocaleString("ja-JP")}`,
            "sub",
          ),
        );
        if (event.detail) element.append(node("p", event.detail));
        if (event.artifact) element.append(node("code", event.artifact));
        element.append(provenance(
          observationView(event.title, event.observation, now),
          undefined, String, "event:" + event.sequence,
        ));
        return element;
      }),
  );
  if (!state.events.length)
    $("events").append(
      node("div", "進捗・成果物の報告はまだありません", "empty"),
    );
}
// Freshness expires even when no new SSE events arrive. Do not rebuild answer forms.
function refreshFreshness() {
  if (lastState && freshnessSignature(lastState, Date.now()) !== reportFreshnessSignature)
    renderReports(lastState);
}
setInterval(refreshFreshness, 30000);
document.addEventListener("visibilitychange", () => {
  if (!document.hidden) refreshFreshness();
});
async function refreshState() {
  try {
    render(await api("state"));
  } catch (e) {
    $("connection").textContent = e.message;
  }
}
let qrObjectUrl = null;
async function renderShare(config) {
  const share = config.share;
  $("share-message").textContent = share.message;
  $("share-url").textContent = share.url || "";
  $("qr").hidden = !share.url;
  $("qr-placeholder").hidden = Boolean(share.url);
  $("qr-placeholder").textContent =
    share.state === "login_required"
      ? "Tailscaleへのログイン待ち"
      : "接続準備中";
  if (qrObjectUrl) URL.revokeObjectURL(qrObjectUrl);
  qrObjectUrl = null;
  if (share.url) {
    const response = await fetch(base + "api/qr.svg?updated=" + Date.now(), {
      headers: browserToken ? { "x-rdsh-browser-token": browserToken } : {},
    });
    if (!response.ok) throw new Error("QRコードを取得できません");
    qrObjectUrl = URL.createObjectURL(await response.blob());
    $("qr").src = qrObjectUrl;
  }
  $("consent").hidden = !share.consent_url;
  if (share.consent_url) $("consent").href = share.consent_url;
  $("mcp-info").textContent =
    config.kind === "project"
      ? `Dotsのイベント購読: ${config.events?.active || 0} 件` +
        (config.events?.failures
          ? ` · 配信エラー ${config.events.failures} 件`
          : "") +
        (config.mcp_url
          ? ` · MCP接続先: ${config.mcp_url}`
          : " · 外部接続は設定待ち")
      : "";
}
$("share-toggle").addEventListener("click", () => {
  $("share").hidden = !$("share").hidden;
  $("share-toggle").setAttribute("aria-expanded", String(!$("share").hidden));
});
$("share-refresh").addEventListener("click", async () => {
  $("share-refresh").disabled = true;
  try {
    await api("share/refresh", {});
    await renderShare(await api("config"));
  } catch (e) {
    $("share-message").textContent = e.message;
  } finally {
    $("share-refresh").disabled = false;
  }
});
try {
  const config = await api("config");
  await renderShare(config);
  if (config.kind === "harness") {
    $("kind").textContent = "DEEPSEEK HARNESS";
    $("title").textContent = "Harnessを開く";
    $("location").textContent = "会話・ツール実行のWeb画面";
    $("project-content").hidden = true;
    $("harness").hidden = false;
    $("harness-open").href = config.harness_url;
    $("connection").textContent = "接続済み · Harness専用の入口";
    $("share").hidden = false;
    $("share-toggle").setAttribute("aria-expanded", "true");
  } else {
    $("title").textContent = config.project.name;
    $("location").textContent = config.project.root;
    document.title = config.project.name + " · Project dashboard";
    await refreshState();
    const source = new EventSource(base + "api/live?key=" + encodeURIComponent(browserToken));
    source.addEventListener("changed", refreshState);
    source.onerror = () => {
      $("connection").textContent = "再接続中…";
    };
    source.onopen = refreshState;
  }
} catch (e) {
  $("connection").textContent = e.message;
}
