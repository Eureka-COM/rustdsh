#!/usr/bin/env node
import { createHash } from 'node:crypto'
import { readFileSync, realpathSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { spawnSync } from 'node:child_process'

const bundle = dirname(fileURLToPath(import.meta.url))
export class PatchError extends Error {
  constructor(code, message) { super(message); this.code = code }
}

function git(source, args, input) {
  const result = spawnSync('git', ['-c', `safe.directory=${source.replaceAll('\\', '/')}`, '-C', source, ...args], {
    encoding: 'utf8', input, windowsHide: true,
  })
  if (result.error) throw new PatchError('GIT_UNAVAILABLE', 'Git must be available on PATH.')
  return result
}

function successful(result, code, message) {
  if (result.status !== 0) throw new PatchError(code, message)
  return result.stdout.trim()
}

/** Apply only the bundled board patch to an explicitly selected, compatible source checkout. */
export function prepare(sourcePath, { apply = false, patch, manifest } = {}) {
  patch ??= readFileSync(resolve(bundle, 'workflow-board.patch'))
  manifest ??= JSON.parse(readFileSync(resolve(bundle, 'manifest.json'), 'utf8'))
  if (createHash('sha256').update(patch).digest('hex') !== manifest.patch_sha256) {
    throw new PatchError('PATCH_INTEGRITY', 'The board patch checksum does not match its manifest.')
  }
  let source
  try { source = realpathSync(sourcePath) } catch {
    throw new PatchError('SOURCE_MISSING', 'Select an existing DSH source checkout with --source.')
  }
  const top = successful(git(source, ['rev-parse', '--show-toplevel']), 'SOURCE_NOT_GIT', 'Select a DSH Git source checkout, not an installed package or profile.')
  if (realpathSync(top) !== source) throw new PatchError('SOURCE_NOT_ROOT', '--source must select the checkout root.')
  const head = successful(git(source, ['rev-parse', 'HEAD']), 'HEAD_UNAVAILABLE', 'The source HEAD cannot be read.')
  if (head !== manifest.base_commit) {
    throw new PatchError('UNSUPPORTED_BASE', 'This source revision is not the verified board base. Do not apply it to installed DSH or a different upstream revision.')
  }
  const stats = successful(git(source, ['apply', '--numstat', '-'], patch), 'PATCH_INVALID', 'The board patch is not a valid Git patch.')
  const paths = stats.split('\n').map(line => line.split('\t').slice(2).join('\t')).sort()
  if (JSON.stringify(paths) !== JSON.stringify([...manifest.files].sort())) {
    throw new PatchError('PATCH_SCOPE', 'The patch file set does not match the board manifest.')
  }
  const outcome = state => ({ state, base_commit: head, board_commit: manifest.board_commit, files: paths.length })
  if (git(source, ['apply', '--reverse', '--check', '-'], patch).status === 0) return outcome('already-applied')
  const dirty = successful(git(source, ['status', '--porcelain', '--untracked-files=normal']), 'STATUS_UNAVAILABLE', 'The source working tree cannot be inspected.')
  if (dirty !== '') throw new PatchError('SOURCE_DIRTY', 'Use a clean isolated DSH checkout; existing edits and untracked files are preserved.')
  successful(git(source, ['apply', '--check', '--whitespace=error-all', '-'], patch), 'PATCH_CONFLICT', 'The board patch does not apply cleanly; no files were changed.')
  if (!apply) return outcome('ready')
  successful(git(source, ['apply', '--whitespace=error-all', '-'], patch), 'APPLY_FAILED', 'Git could not apply the board patch. Inspect this source checkout before retrying.')
  successful(git(source, ['apply', '--reverse', '--check', '-'], patch), 'VERIFY_FAILED', 'The patched source failed verification; inspect the source diff before building.')
  return outcome('applied')
}

export function main(args) {
  let source, apply = false
  for (let index = 0; index < args.length; index++) {
    const arg = args[index]
    if (arg === '--source') source = args[++index]
    else if (arg === '--apply') apply = true
    else if (arg === '--check') apply = false
    else if (arg === '--help') {
      console.log('node plugins/workflow-board/source-patch.mjs --source <clean-dsh-checkout> [--check|--apply]')
      console.log('Default: check only. Applies source files; never installs, builds, changes profiles, or starts tasks.')
      return
    } else throw new PatchError('USAGE', 'Use --source <checkout> and --check or --apply.')
  }
  if (!source) throw new PatchError('USAGE', '--source <DSH source checkout> is required; default mode is read-only.')
  console.log(JSON.stringify(prepare(source, { apply })))
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { main(process.argv.slice(2)) } catch (error) {
    const known = error instanceof PatchError
    console.error(JSON.stringify({ error: known ? error.code : 'UNEXPECTED', message: known ? error.message : 'Cannot prepare this source; no installation was attempted.' }))
    process.exitCode = 1
  }
}
