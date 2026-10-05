import { homedir } from 'node:os';
import { join } from 'node:path';
import { readFile, writeFile, mkdir } from 'node:fs/promises';

export const inject = ['webServer'];

function dshHome() {
  return process.env.DSH_HOME || join(homedir(), '.dsh');
}
function cfgPath() { return join(dshHome(), 'rdsh-context.json'); }
function settingsPath() { return join(dshHome(), 'rdsh.json'); }

function defaults() {
  return { token_budget: 4000, enable_retriever: true, enable_packer: true, enable_verifier: true, goal: '', decisions: [], constraints: [], working_files: [], open_tasks: [] };
}

function settingsDefaults() {
  return {
    schema: 1,
    general: { slim: true, passthrough: false, dry_run: false, default_profile: '' },
    tokens: { default_budget: 4000 },
    search: { dir: '.', max: 100, web_limit: 10, searxng_url: '' },
    compact: { max_tokens: 8000 },
    sessions: { limit: 20, with_tokens: false },
    logs: { tail: 50 },
    serve: { port: 3080 },
    guard: { deny: [], reason: '' },
    bench: { n: 5 },
    setup: { web_port: 0 },
    beta: { context_engine: true },
    context: { token_budget: 4000, enable_retriever: true, enable_packer: true, enable_verifier: true, goal: '', decisions: [], constraints: [], working_files: [], open_tasks: [], max_code_hits: 20, max_sessions: 10, include_git_diff: true },
  };
}

async function loadCfg() {
  try {
    const raw = await readFile(cfgPath(), 'utf8');
    const j = JSON.parse(raw);
    const d = defaults();
    return sanitize({ ...d, ...j });
  } catch (e) { return defaults(); }
}

async function loadSettings() {
  try {
    const raw = await readFile(settingsPath(), 'utf8');
    const j = JSON.parse(raw);
    return sanitizeSettings(j && typeof j === 'object' ? j : {});
  } catch (e) { return settingsDefaults(); }
}

function strList(v, maxN, maxC) {
  if (!Array.isArray(v)) return [];
  return v.filter((x) => typeof x === 'string').map((s) => s.slice(0, maxC)).filter((s) => s.trim() !== '').slice(0, maxN);
}

function sanitize(j) {
  const d = defaults();
  const out = { ...d };
  const b = Number(j.token_budget);
  out.token_budget = Number.isFinite(b) ? Math.min(200000, Math.max(500, Math.round(b))) : 4000;
  for (const k of ['enable_retriever', 'enable_packer', 'enable_verifier']) {
    if (typeof j[k] === 'boolean') out[k] = j[k];
  }
  if (typeof j.goal === 'string') out.goal = j.goal.slice(0, 2000);
  out.decisions = strList(j.decisions, 50, 500);
  out.constraints = strList(j.constraints, 50, 500);
  out.working_files = strList(j.working_files ?? j.files, 50, 300);
  out.open_tasks = strList(j.open_tasks, 50, 500);
  return out;
}

function clampInt(v, min, max, fb) {
  const n = Number(v);
  if (!Number.isFinite(n)) return fb;
  return Math.min(max, Math.max(min, Math.round(n)));
}

function bool(v, fb) {
  return typeof v === 'boolean' ? v : fb;
}

function str(v, maxC, fb) {
  return typeof v === 'string' ? v.slice(0, maxC) : fb;
}

function obj(v) {
  return v && typeof v === 'object' && !Array.isArray(v) ? v : {};
}

function sanitizeSettings(j) {
  const d = settingsDefaults();
  const src = obj(j);
  const g = obj(src.general); const dg = d.general;
  const tk = obj(src.tokens);
  const se = obj(src.search);
  const co = obj(src.compact);
  const ss = obj(src.sessions);
  const lg = obj(src.logs);
  const sv = obj(src.serve);
  const gu = obj(src.guard);
  const be = obj(src.bench);
  const su = obj(src.setup);
  const bt = obj(src.beta);
  const cx = obj(src.context);
  return {
    schema: 1,
    general: {
      slim: bool(g.slim, dg.slim),
      passthrough: bool(g.passthrough, dg.passthrough),
      dry_run: bool(g.dry_run, dg.dry_run),
      default_profile: str(g.default_profile, 500, dg.default_profile),
    },
    tokens: {
      default_budget: clampInt(tk.default_budget, 500, 200000, d.tokens.default_budget),
    },
    search: {
      dir: str(se.dir, 300, d.search.dir),
      max: clampInt(se.max, 1, 100, d.search.max),
      web_limit: clampInt(se.web_limit, 1, 100, d.search.web_limit),
      searxng_url: str(se.searxng_url, 500, d.search.searxng_url),
    },
    compact: {
      max_tokens: clampInt(co.max_tokens, 500, 200000, d.compact.max_tokens),
    },
    sessions: {
      limit: clampInt(ss.limit, 1, 100, d.sessions.limit),
      with_tokens: bool(ss.with_tokens, d.sessions.with_tokens),
    },
    logs: {
      tail: clampInt(lg.tail, 1, 500, d.logs.tail),
    },
    serve: {
      port: clampInt(sv.port, 1, 65535, d.serve.port),
    },
    guard: {
      deny: strList(gu.deny, 50, 300),
      reason: str(gu.reason, 500, d.guard.reason),
    },
    bench: {
      n: clampInt(be.n, 1, 20, d.bench.n),
    },
    setup: {
      web_port: clampInt(su.web_port, 0, 65535, d.setup.web_port),
    },
    beta: {
      context_engine: bool(bt.context_engine, d.beta.context_engine),
    },
    context: {
      token_budget: clampInt(cx.token_budget, 500, 200000, d.context.token_budget),
      enable_retriever: bool(cx.enable_retriever, d.context.enable_retriever),
      enable_packer: bool(cx.enable_packer, d.context.enable_packer),
      enable_verifier: bool(cx.enable_verifier, d.context.enable_verifier),
      goal: str(cx.goal, 2000, d.context.goal),
      decisions: strList(cx.decisions, 50, 500),
      constraints: strList(cx.constraints, 50, 500),
      working_files: strList(cx.working_files ?? cx.files, 50, 300),
      open_tasks: strList(cx.open_tasks, 50, 500),
      max_code_hits: clampInt(cx.max_code_hits, 1, 100, d.context.max_code_hits),
      max_sessions: clampInt(cx.max_sessions, 1, 100, d.context.max_sessions),
      include_git_diff: bool(cx.include_git_diff, d.context.include_git_diff),
    },
  };
}

async function readBody(req) {
  let size = 0; const chunks = [];
  for await (const chunk of req) { size += chunk.length; if (size > 65536) throw new Error('too large'); chunks.push(chunk); }
  return JSON.parse(Buffer.concat(chunks).toString('utf8') || '{}');
}

function json(res, code, body) {
  const s = JSON.stringify(body);
  res.writeHead(code, { 'content-type': 'application/json; charset=utf-8', 'cache-control': 'no-store', 'content-length': Buffer.byteLength(s) });
  res.end(s);
}

export function apply(ctx, config) {
  return ctx.effect(() => {
    if (!ctx.webServer) return;
    const offGet = ctx.webServer.register({
      kind: 'exact',
      path: '/api/rdsh-context',
      handler: async (req, res) => {
        if (req.method !== 'GET' && req.method !== 'HEAD') {
          res.writeHead(405, { 'content-type': 'application/json' });
          res.end(JSON.stringify({ error: 'method-not-allowed' }));
          return;
        }
        const cfg = await loadCfg();
        const body = JSON.stringify({ ok: true, prototype: true, config: cfg });
        res.writeHead(200, { 'content-type': 'application/json; charset=utf-8', 'cache-control': 'no-store', 'content-length': Buffer.byteLength(body) });
        res.end(body);
      },
    });
    const offPost = ctx.webServer.register({
      kind: 'exact',
      path: '/api/rdsh-context/save',
      handler: async (req, res) => {
        if (req.method !== 'POST') {
          res.writeHead(405, { 'content-type': 'application/json' });
          res.end(JSON.stringify({ error: 'method-not-allowed' }));
          return;
        }
        try {
          const input = await readBody(req);
          const cfg = sanitize(input.config ?? input);
          await mkdir(dshHome(), { recursive: true });
          await writeFile(cfgPath(), JSON.stringify(cfg, null, 2) + '\n', { mode: 0o600 });
          const body = JSON.stringify({ ok: true, config: cfg });
          res.writeHead(200, { 'content-type': 'application/json; charset=utf-8', 'cache-control': 'no-store', 'content-length': Buffer.byteLength(body) });
          res.end(body);
        } catch (e) {
          res.writeHead(400, { 'content-type': 'application/json' });
          res.end(JSON.stringify({ ok: false, error: 'bad-request' }));
        }
      },
    });
    const offGetSettings = ctx.webServer.register({
      kind: 'exact',
      path: '/api/rdsh-settings',
      handler: async (req, res) => {
        if (req.method !== 'GET' && req.method !== 'HEAD') {
          res.writeHead(405, { 'content-type': 'application/json' });
          res.end(JSON.stringify({ error: 'method-not-allowed' }));
          return;
        }
        const cfg = await loadSettings();
        json(res, 200, { ok: true, config: cfg });
      },
    });
    const offPostSettings = ctx.webServer.register({
      kind: 'exact',
      path: '/api/rdsh-settings/save',
      handler: async (req, res) => {
        if (req.method !== 'POST') {
          res.writeHead(405, { 'content-type': 'application/json' });
          res.end(JSON.stringify({ error: 'method-not-allowed' }));
          return;
        }
        try {
          const input = await readBody(req);
          const cfg = sanitizeSettings(input.config ?? input);
          await mkdir(dshHome(), { recursive: true });
          await writeFile(settingsPath(), JSON.stringify(cfg, null, 2) + '\n', { mode: 0o600 });
          json(res, 200, { ok: true, config: cfg });
        } catch (e) {
          res.writeHead(400, { 'content-type': 'application/json' });
          res.end(JSON.stringify({ ok: false, error: 'bad-request' }));
        }
      },
    });
    return () => { try { if (typeof offGet === 'function') offGet(); } catch (e) {} try { if (typeof offPost === 'function') offPost(); } catch (e) {} try { if (typeof offGetSettings === 'function') offGetSettings(); } catch (e) {} try { if (typeof offPostSettings === 'function') offPostSettings(); } catch (e) {} };
  });
}
