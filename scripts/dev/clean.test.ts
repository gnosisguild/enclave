// SPDX-License-Identifier: LGPL-3.0-only

import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { test } from 'node:test'

import { clean } from './clean'

function fixture(t: { after: (fn: () => void) => void }) {
  const root = mkdtempSync(join(tmpdir(), 'interfold-clean-'))
  t.after(() => rmSync(root, { recursive: true, force: true }))
  const write = (path: string, contents = path) => {
    mkdirSync(dirname(join(root, path)), { recursive: true })
    writeFileSync(join(root, path), contents)
  }
  execFileSync('git', ['init', '--quiet'], { cwd: root })
  for (const path of [
    'package.json',
    'Cargo.toml',
    'packages/contracts/hardhat.config.ts',
    'crates/a/tests/fixtures/dummy.json',
    'packages/contracts/artifacts/contracts/Tracked.json',
    'target/staged.txt',
  ])
    write(path)
  execFileSync('git', ['add', '.'], { cwd: root })
  for (const path of [
    'dist/build.js',
    'target/build/a',
    'packages/contracts/artifacts/contracts/Untracked.json',
    '.interfold/data/key',
    '.interfold/config/config.yml',
    'database/data',
    'packages/contracts/broadcast/tx.json',
    'packages/contracts/ignition/deployments/state.json',
    'dist/database/operator',
    'dist/nested/.git/HEAD',
    'dist/nested/valuable',
  ])
    write(path)
  return { root, write, has: (path: string) => existsSync(join(root, path)) }
}

test('clean removes rebuildable output but protects indexed files, their parents, state and nested repositories', async (t) => {
  const { root, has } = fixture(t)
  await clean(root)
  for (const path of [
    'crates/a/tests/fixtures/dummy.json',
    'packages/contracts/artifacts/contracts/Tracked.json',
    'target/staged.txt',
    '.interfold/data/key',
    '.interfold/config/config.yml',
    'database/data',
    'packages/contracts/broadcast/tx.json',
    'packages/contracts/ignition/deployments/state.json',
    'dist/database/operator',
    'dist/nested/valuable',
  ])
    assert.ok(has(path), path)
  for (const path of ['dist/build.js', 'target/build', 'packages/contracts/artifacts/contracts/Untracked.json'])
    assert.equal(has(path), false, path)
})

test('dry run changes nothing and skip flags preserve the selected output groups', async (t) => {
  const { root, has } = fixture(t)
  await clean(root, { dryRun: true })
  assert.ok(has('dist/build.js'))
  assert.ok(has('target/build/a'))
  assert.ok(has('packages/contracts/artifacts/contracts/Untracked.json'))
  await clean(root, { skipNode: true, skipCrates: true, skipContracts: true })
  assert.ok(has('dist/build.js'))
  assert.ok(has('target/build/a'))
  assert.ok(has('packages/contracts/artifacts/contracts/Untracked.json'))
})

test('clean does not follow output symlinks or manifest ancestors into another directory', async (t) => {
  const { root, write } = fixture(t)
  const outside = mkdtempSync(join(tmpdir(), 'interfold-clean-outside-'))
  t.after(() => rmSync(outside, { recursive: true, force: true }))
  writeFileSync(join(outside, 'valuable'), 'keep')
  symlinkSync(outside, join(root, 'dist/link'))
  symlinkSync(join(outside, 'does-not-exist'), join(root, 'dist/broken-link'))
  write('nested/package.json')
  execFileSync('git', ['add', 'nested/package.json'], { cwd: root })
  rmSync(join(root, 'nested'), { recursive: true })
  symlinkSync(outside, join(root, 'nested'))
  mkdirSync(join(outside, 'dist'))
  writeFileSync(join(outside, 'dist/build'), 'keep')
  await clean(root)
  assert.equal(readFileSync(join(outside, 'valuable'), 'utf8'), 'keep')
  assert.equal(readFileSync(join(outside, 'dist/build'), 'utf8'), 'keep')
  assert.ok(existsSync(join(root, 'dist')))
})

for (const [flag, kept, removed] of [
  ['skipNode', 'dist/build.js', 'target/build/a'],
  ['skipCrates', 'target/build/a', 'dist/build.js'],
  ['skipContracts', 'packages/contracts/artifacts/contracts/Untracked.json', 'dist/build.js'],
] as const) {
  test(`${flag} preserves only its build-output group`, async (t) => {
    const { root, has } = fixture(t)
    await clean(root, { [flag]: true })
    assert.ok(has(kept))
    assert.equal(has(removed), false)
  })
}
