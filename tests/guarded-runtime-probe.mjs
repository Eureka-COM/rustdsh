// Used in an isolated Node process with the mandatory preload.
import assert from 'node:assert/strict';
import { pathToFileURL } from 'node:url';
import path from 'node:path';
const root = path.resolve(process.env.RDSH_TOOL_RUNTIME, '../../..');
const { Context } = await import(pathToFileURL(path.join(root, 'cordis/lib/index.js')));
const { default: ToolRuntime } = await import(pathToFileURL(process.env.RDSH_TOOL_RUNTIME));
const ctx = new Context();
ctx.provide('systemPrompt', { tools() {}, section() {}, getSectionOrder() { return 1; } });
const tools = new ToolRuntime(ctx, { mode: 'ptc' });
assert.equal(tools.defaultMode, 'native');
assert.ok(tools.guardReason({ name: 'bash' }));
assert.ok(tools.guardReason({ name: 'run_code' }));
assert.equal(tools.guardReason({ name: 'rdsh_inspect' }), undefined);
const result = await tools.get('rdsh_inspect').execute({ command: 'printf DUMMY_GUARDED_RUNTIME' });
assert.equal(result.exitCode, 0, result.stderr);
assert.equal(result.stdout, 'DUMMY_GUARDED_RUNTIME');
console.log('guarded runtime initialized and inspected with kernel isolation');
