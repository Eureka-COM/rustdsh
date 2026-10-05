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
function render(state) {
  const m = state.metrics,
    done = state.tasks.filter((task) => task.status === "done").length,
    pending = state.questions.filter((question) => question.answer === null),
    answered = state.questions.length - pending.length;
  const cache = ratio(m.cached_input_tokens, m.input_tokens),
    errors = ratio(m.tool_errors, m.tool_calls);
  const counts = ["done", "doing", "todo", "blocked"]
    .map(
      (status) =>
        `${status} ${state.tasks.filter((task) => task.status === status).length}`,
    )
    .join(" · ");
  $("cards").replaceChildren(
    card(
      "費用（API換算、累計）",
      money(m.total_cost_usd),
      m.total_budget_usd == null
        ? "上限 未設定"
        : "上限 " + money(m.total_budget_usd),
      ratio(m.total_cost_usd, m.total_budget_usd),
    ),
    card(
      "直近のセッション",
      money(m.session_cost_usd),
      `${m.session_id || "未取得"}　上限 ${m.session_budget_usd == null ? "未設定" : money(m.session_budget_usd)}`,
      ratio(m.session_cost_usd, m.session_budget_usd),
    ),
    card(
      "キャッシュ読み込み率",
      percentage(cache),
      `${number(m.model_calls)} 回の呼び出し（入力トークン加重）`,
    ),
    card(
      "ツールのエラー率",
      percentage(errors),
      `${number(m.tool_errors)} / ${number(m.tool_calls)} 件`,
    ),
    card(
      "文脈の読み落とし",
      number(m.context_misses),
      "報告元で検出した回数",
      undefined,
      m.context_misses > 0,
    ),
    card(
      "自動続行",
      number(m.auto_continues),
      `拒否 ${number(m.refusals)} · APIエラー ${number(m.api_errors)}`,
    ),
    card("タスク", `${done} / ${state.tasks.length}`, counts),
    card("未回答の質問", String(pending.length), `回答済み ${answered}`),
  );
  $("task-milestones").textContent = [
    ...new Set(state.tasks.map((task) => task.milestone).filter(Boolean)),
  ].join(" / ");
  $("tasks").replaceChildren(
    ...state.tasks.map((task) => {
      const tr = node("tr");
      const status = node("td");
      status.append(node("span", task.status, "status " + task.status));
      const title = node("td", task.title);
      const versions = state.contracts?.find((item) => item.task_id === task.id)?.versions || [];
      const current = versions.at(-1);
      const details = node("details");
      details.append(node("summary", current ? `タスク契約 v${current.version}` : "タスク契約 未設定"));
      if (current) {
        for (const contract of versions.slice().reverse()) {
          details.append(node("strong", `v${contract.version} ・ ${contract.changed_at}`));
          for (const [label, value] of [
            ["目的", contract.purpose],
            ["対象repo", contract.repository],
            ["許可範囲", contract.allowed_scope],
            ["書込先", contract.write_roots.join("\n") || "書込不可"],
            ["読取先", contract.operation_policy?.read_roots.join("\n") || "構造policyでは未許可"],
            ["実行file", contract.operation_policy?.executables.map((rule) => `${rule.file} (${rule.argument_count}引数・hash照合)`).join("\n") || "未許可"],
            ["network origin", contract.operation_policy?.network_origins.join("\n") || "未許可"],
            ["禁止事項", contract.forbidden_actions.join("\n") || "記載なし"],
            ["終了条件", contract.completion_conditions.join("\n")],
            ["変更理由", contract.change_reason],
          ]) details.append(node("p", `${label}: ${value}`));
        }
        details.append(node("p", "契約変更後は承認の再照合が必要です。質問への回答は契約を変更しません。"));
      }
      title.append(details);
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
  $("policy-checks").replaceChildren(
    node("p", "文字列filter・操作構造の照合・実環境の強制は別の層です。shell構文は未対応、sandboxは未適用です。", "notice"),
    ...(state.policy_checks || []).slice(-10).reverse().map((check) => {
      const element = node("article", undefined, "event");
      element.append(node("strong", `${check.id}: ${check.decision}`),
        node("p", `${check.reason} ・ 実行: ${check.execution}`));
      return element;
    }),
  );
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
        return element;
      }),
  );
  if (!state.events.length)
    $("events").append(
      node("div", "進捗・成果物の報告はまだありません", "empty"),
    );
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
    `最終更新: ${state.updated_at ? new Date(state.updated_at).toLocaleString("ja-JP") : "まだ報告がありません"} · 未取得の指標はMCPから報告されたときに表示されます。費用は報告元のAPI換算値です。`;
}
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
