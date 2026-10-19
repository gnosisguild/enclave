// SPDX-License-Identifier: LGPL-3.0-only

import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { test } from 'node:test'

test('CRISP gas setup builds cold dependencies, preserves node artifacts, and restores the pair on failure', (t) => {
  const dir = mkdtempSync(join(tmpdir(), 'interfold-crisp-gas-'))
  t.after(() => rmSync(dir, { recursive: true, force: true }))
  for (const path of ['scripts/circuits', 'bin', 'circuits/bin', 'dist/circuits/insecure-64/minimum', 'examples/CRISP/scripts'])
    mkdirSync(join(dir, path), { recursive: true })
  copyFileSync(new URL('prepare-crisp-gas.mjs', import.meta.url), join(dir, 'scripts/circuits/prepare-crisp-gas.mjs'))
  copyFileSync(
    new URL('../../examples/CRISP/scripts/build-presets.mjs', import.meta.url),
    join(dir, 'examples/CRISP/scripts/build-presets.mjs'),
  )
  writeFileSync(join(dir, 'examples/CRISP/scripts/stage-preset-artifacts.mjs'), '')
  writeFileSync(join(dir, 'bin/bash'), '#!/bin/sh\nexit 0\n', { mode: 0o755 })
  writeFileSync(
    join(dir, 'bin/pnpm'),
    `#!${process.execPath}
const fs = require('node:fs')
const path = require('node:path')
const { execFileSync } = require('node:child_process')
const root = process.env.FIXTURE_ROOT
const args = process.argv.slice(2)
const write = (file, value = '') => { fs.mkdirSync(path.dirname(path.join(root, file)), { recursive: true }); fs.writeFileSync(path.join(root, file), value) }
const fail = (stage) => { if (process.env.FAIL_STAGE === stage) process.exit(17) }
const requireFile = (file) => { if (!fs.existsSync(path.join(root, file))) { console.error('Missing dependency: ' + file); process.exit(19) } }
if (args[0] === 'wasm:build') { fail('wasm'); write('wasm-ready') }
else if (args.includes('build:sdk')) {
  requireFile('wasm-ready')
  execFileSync(process.execPath, [path.join(root, 'examples/CRISP/scripts/build-presets.mjs')], { stdio: 'inherit' })
  if (process.env.INTERRUPT_SIGNAL) process.kill(process.ppid, process.env.INTERRUPT_SIGNAL)
  fail('sdk')
  write('sdk-ready')
} else if (args[0] === 'build:circuits') {
  const preset = args[args.indexOf('--preset') + 1]
  const committee = args.includes('--committee') ? args[args.indexOf('--committee') + 1] : 'minimum'
  if (args.includes('--hydrate-bin-only')) { fail('restore'); write('restored') }
  else {
    const output = args.includes('--output') ? args[args.indexOf('--output') + 1] : path.join(root, 'dist/circuits')
    fs.mkdirSync(path.join(output, preset, committee), { recursive: true })
    fs.writeFileSync(path.join(output, preset, committee, 'node-artifact'), 'partial threshold build')
  }
  write('circuits/bin/.active-preset.json', JSON.stringify({ preset, committee }))
} else if (args[0] === 'evm:build') {
  for (const file of ['wasm-ready', 'sdk-ready', 'restored']) requireFile(file)
  fail('evm')
  write('packages/interfold-contracts/dist/tasks/utils.js')
} else if (args[0] === 'test:ballots:program') {
  for (const file of ['wasm-ready', 'sdk-ready', 'packages/interfold-contracts/dist/tasks/utils.js']) requireFile(file)
} else process.exit(23)
`,
    { mode: 0o755 },
  )
  const active = join(dir, 'circuits/bin/.active-preset.json')
  const archived = join(dir, 'dist/circuits/insecure-64/minimum/node-artifact')
  const env = { ...process.env, FIXTURE_ROOT: dir, PATH: `${dir}/bin:${process.env.PATH}` }
  const ballot = () => spawnSync(join(dir, 'bin/pnpm'), ['test:ballots:program'], { cwd: dir, env, encoding: 'utf8' })
  assert.equal(ballot().status, 19, 'the cold ballot test must reject missing dependencies')
  for (const failed of ['', 'wasm', 'sdk', 'restore', 'evm']) {
    for (const file of ['wasm-ready', 'sdk-ready', 'restored', 'packages/interfold-contracts/dist/tasks/utils.js'])
      rmSync(join(dir, file), { force: true })
    writeFileSync(active, JSON.stringify({ preset: 'secure-8192', committee: 'micro' }))
    writeFileSync(archived, 'complete published node pair')
    const result = spawnSync(process.execPath, ['scripts/circuits/prepare-crisp-gas.mjs'], {
      cwd: dir,
      env: { ...env, FAIL_STAGE: failed },
      encoding: 'utf8',
    })
    assert.equal(result.status, failed ? 1 : 0, result.stdout + result.stderr)
    assert.equal(readFileSync(archived, 'utf8'), 'complete published node pair')
    if (failed !== 'restore') assert.deepEqual(JSON.parse(readFileSync(active)), { preset: 'secure-8192', committee: 'micro' })
    if (!failed) assert.equal(ballot().status, 0)
    else assert.equal(existsSync(join(dir, 'packages/interfold-contracts/dist/tasks/utils.js')), false)
  }
  for (const [signal, code] of [
    ['SIGINT', 130],
    ['SIGTERM', 143],
  ]) {
    writeFileSync(active, JSON.stringify({ preset: 'secure-8192', committee: 'micro' }))
    const result = spawnSync(process.execPath, ['scripts/circuits/prepare-crisp-gas.mjs'], {
      cwd: dir,
      env: { ...env, INTERRUPT_SIGNAL: signal },
      encoding: 'utf8',
      timeout: 15000,
    })
    assert.equal(result.status, code, result.stdout + result.stderr)
    assert.deepEqual(JSON.parse(readFileSync(active)), { preset: 'secure-8192', committee: 'micro' })
  }
})
