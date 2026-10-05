import test from 'node:test'
import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { mkdtempSync, writeFileSync, readFileSync, rmSync, mkdirSync, realpathSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve, dirname, sep } from 'node:path'
import { fileURLToPath } from 'node:url'
import { execFileSync, spawnSync } from 'node:child_process'
import { prepare } from '../source-patch.mjs'

const bundle = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const hash = input => createHash('sha256').update(input).digest('hex')
function removeFixture(source) {
  const root = realpathSync(tmpdir())
  const target = realpathSync(source)
  assert.ok(target.startsWith(root + sep), 'fixture deletion must stay inside the temporary directory')
  rmSync(target, { recursive: true, force: true })
}
function fixture(t) {
  const source = mkdtempSync(join(tmpdir(), 'rdsh board fixture '))
  t.after(() => removeFixture(source))
  const git = (...args) => execFileSync('git', ['-C', source, ...args], { encoding: 'utf8', windowsHide: true }).trim()
  git('init', '-q')
  writeFileSync(join(source, '.gitattributes'), '* text=auto eol=lf\n')
  writeFileSync(join(source, 'card.txt'), 'started\n')
  git('add', '.gitattributes', 'card.txt')
  git('-c', 'user.name=Fixture', '-c', 'user.email=fixture@local.invalid', '-c', 'core.hooksPath=/dev/null', 'commit', '-qm', 'fixture base')
  const base_commit = git('rev-parse', 'HEAD')
  writeFileSync(join(source, 'card.txt'), 'started\ncompleted\n')
  const patch = Buffer.from(git('diff', '--binary') + '\n')
  git('restore', 'card.txt')
  const manifest = { base_commit, board_commit: 'fixture', patch_sha256: hash(patch), files: ['card.txt'] }
  return { source, patch, manifest, git }
}

test('checks without writing; applies to a checkout with spaces; repeated application is inert', t => {
  const f = fixture(t)
  assert.equal(prepare(f.source, f).state, 'ready')
  assert.equal(readFileSync(join(f.source, 'card.txt'), 'utf8'), 'started\n')
  assert.equal(f.git('status', '--porcelain'), '')
  assert.equal(prepare(f.source, { ...f, apply: true }).state, 'applied')
  assert.equal(readFileSync(join(f.source, 'card.txt'), 'utf8'), 'started\ncompleted\n')
  assert.equal(prepare(f.source, { ...f, apply: true }).state, 'already-applied')
})

test('rejects modified source, untracked files, wrong roots and unsupported revisions', t => {
  const f = fixture(t)
  writeFileSync(join(f.source, 'card.txt'), 'user edit\n')
  assert.throws(() => prepare(f.source, { ...f, apply: true }), { code: 'SOURCE_DIRTY' })
  assert.equal(readFileSync(join(f.source, 'card.txt'), 'utf8'), 'user edit\n')
  f.git('restore', 'card.txt')
  writeFileSync(join(f.source, 'untracked.txt'), 'keep\n')
  assert.throws(() => prepare(f.source, f), { code: 'SOURCE_DIRTY' })
  rmSync(join(f.source, 'untracked.txt'))
  mkdirSync(join(f.source, 'subdir'))
  assert.throws(() => prepare(join(f.source, 'subdir'), f), { code: 'SOURCE_NOT_ROOT' })
  assert.throws(() => prepare(f.source, { ...f, manifest: { ...f.manifest, base_commit: 'different' } }), { code: 'UNSUPPORTED_BASE' })
})

test('rejects changed patch bytes and unexpected paths before writing', t => {
  const f = fixture(t)
  assert.throws(() => prepare(f.source, { ...f, patch: Buffer.from('tampered') }), { code: 'PATCH_INTEGRITY' })
  assert.throws(() => prepare(f.source, { ...f, manifest: { ...f.manifest, files: ['another.txt'] } }), { code: 'PATCH_SCOPE' })
  assert.equal(f.git('status', '--porcelain'), '')
})

test('a conflicting multi-file patch applies no partial changes', t => {
  const f = fixture(t)
  const patch = Buffer.from(f.patch.toString() + 'diff --git a/missing.txt b/missing.txt\n--- a/missing.txt\n+++ b/missing.txt\n@@ -1 +1 @@\n-old\n+new\n')
  const manifest = { ...f.manifest, patch_sha256: hash(patch), files: ['card.txt', 'missing.txt'] }
  assert.throws(() => prepare(f.source, { patch, manifest, apply: true }), { code: 'PATCH_CONFLICT' })
  assert.equal(readFileSync(join(f.source, 'card.txt'), 'utf8'), 'started\n')
  assert.equal(f.git('status', '--porcelain'), '')
})

test('bundled patch is pinned to the board commit, exact base and only 18 UI/lock paths', () => {
  const manifest = JSON.parse(readFileSync(join(bundle, 'manifest.json'), 'utf8'))
  const patch = readFileSync(join(bundle, 'workflow-board.patch'))
  assert.equal(hash(patch), manifest.patch_sha256)
  assert.equal(manifest.base_commit, 'f97c0438fb1608bbc4c08c88a27344249795ea22')
  assert.equal(manifest.board_commit, '7bd9ac31c23fe369d1fa9a6849869f04967d6ee5')
  assert.equal(manifest.files.length, 18)
  assert.equal(new Set(manifest.files).size, 18)
  assert.ok(manifest.files.every(path => path === 'pnpm-lock.yaml' || path.startsWith('packages/client/ui-workflow-run/')))
  assert.ok(readFileSync(join(bundle, 'DSH-LICENSE'), 'utf8').includes('MIT License'))
})

test('CLI diagnostics reject missing source and installed-package-shaped directories', t => {
  const f = fixture(t)
  const invoke = args => spawnSync(process.execPath, [join(bundle, 'source-patch.mjs'), ...args], { encoding: 'utf8', windowsHide: true })
  const absent = invoke([])
  assert.equal(absent.status, 1)
  assert.equal(JSON.parse(absent.stderr).error, 'USAGE')
  const installed = mkdtempSync(join(tmpdir(), 'rdsh installed fixture '))
  t.after(() => removeFixture(installed))
  writeFileSync(join(installed, 'package.json'), '{"name":"@deepseek-ai/dsh","version":"0.2.0-rc.2"}')
  const result = invoke(['--source', installed, '--check'])
  assert.equal(result.status, 1)
  assert.equal(JSON.parse(result.stderr).error, 'SOURCE_NOT_GIT')
  assert.equal(readFileSync(join(installed, 'package.json'), 'utf8'), '{"name":"@deepseek-ai/dsh","version":"0.2.0-rc.2"}')
})
