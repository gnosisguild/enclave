// SPDX-License-Identifier: LGPL-3.0-only

import { spawn } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
let interrupted
let active
for (const signal of ['SIGINT', 'SIGTERM']) {
  process.on(signal, () => {
    if (interrupted) return
    interrupted = signal
    if (active) {
      try {
        if (process.platform === 'win32') active.kill(signal)
        else process.kill(-active.pid, signal)
      } catch (error) {
        if (error.code !== 'ESRCH') console.error(error)
      }
    }
  })
}
const run = async (args, restoring = false) => {
  if (interrupted && !restoring) throw new Error(`CRISP setup interrupted by ${interrupted}`)
  const child = spawn('pnpm', args, { cwd: root, stdio: 'inherit', detached: process.platform !== 'win32' })
  if (!restoring) active = child
  try {
    await new Promise((resolve, reject) => {
      child.once('error', reject)
      child.once('close', (code, signal) => {
        if (code === 0) resolve()
        else reject(new Error(`pnpm ${args.join(' ')} failed (${signal || code})`))
      })
    })
  } finally {
    active = undefined
  }
}

try {
  // The SDK build selects insecure-64. Restore the node pair before compiling its contracts.
  const { preset, committee } = JSON.parse(readFileSync(join(root, 'circuits/bin/.active-preset.json'), 'utf8'))
  if (!preset || !committee) throw new Error('Select a complete node artifact pair with pnpm build:circuits first.')
  let buildError
  try {
    await run(['wasm:build'])
    await run(['-C', 'examples/CRISP', 'build:sdk'])
  } catch (error) {
    buildError = error
  }
  try {
    await run(['build:circuits', '--preset', preset, '--committee', committee, '--hydrate-bin-only'], true)
  } catch (error) {
    throw new AggregateError([buildError, error].filter(Boolean), 'Cannot restore the node artifact pair after CRISP setup.')
  }
  if (buildError) throw buildError
  // CRISP's Hardhat configuration imports the compiled tasks, not the Solidity-only build.
  await run(['evm:build'])
} catch (error) {
  console.error(error)
  process.exitCode = 1
} finally {
  if (interrupted) process.exitCode = interrupted === 'SIGINT' ? 130 : 143
}
