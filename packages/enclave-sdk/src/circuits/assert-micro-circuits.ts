// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { existsSync, readFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'

import { SDKError } from '../utils'

/** Matches `IEnclave.CommitteeSize.Micro` and `DEFAULT_E3_CONFIG.committeeSize`. */
export const SDK_CIRCUIT_COMMITTEE = 'micro'

const ACTIVE_PRESET_RELATIVE_PATH = 'circuits/bin/.active-preset.json'

function findActivePresetPath(startDir: string): string | undefined {
  let currentDir = resolve(startDir)

  while (true) {
    const activePresetPath = join(currentDir, ACTIVE_PRESET_RELATIVE_PATH)

    if (existsSync(activePresetPath)) return activePresetPath

    if (existsSync(join(currentDir, 'pnpm-workspace.yaml')) && existsSync(join(currentDir, 'circuits'))) {
      return activePresetPath
    }

    const parentDir = dirname(currentDir)
    if (parentDir === currentDir) return undefined

    currentDir = parentDir
  }
}

function resolveActivePresetPath(): string {
  const currentWorkingDir = typeof process !== 'undefined' && typeof process.cwd === 'function' ? process.cwd() : undefined

  return (
    (currentWorkingDir ? findActivePresetPath(currentWorkingDir) : undefined) ??
    resolve(currentWorkingDir ?? '.', ACTIVE_PRESET_RELATIVE_PATH)
  )
}

let checked = false

/**
 * SDK encryption artifacts are built for the micro committee preset by default.
 * Fail fast when `circuits/bin/.active-preset.json` points at another committee
 * (e.g. after benchmark runs with `--committee small`).
 */
export function assertSdkMicroCircuits(): void {
  if (checked) return

  const activePresetPath = resolveActivePresetPath()

  let raw: string
  try {
    raw = readFileSync(activePresetPath, 'utf-8')
  } catch {
    throw new SDKError(
      `Missing ${activePresetPath}. Run \`pnpm -C packages/enclave-sdk compile:circuits\` first.`,
      'SDK_CIRCUIT_STAMP_MISSING',
    )
  }

  let committee: string | undefined
  try {
    committee = JSON.parse(raw)?.committee as string | undefined
  } catch {
    throw new SDKError(
      `Invalid JSON in ${activePresetPath}. Rebuild with \`pnpm -C packages/enclave-sdk compile:circuits\`.`,
      'SDK_CIRCUIT_STAMP_INVALID',
    )
  }

  if (committee !== SDK_CIRCUIT_COMMITTEE) {
    throw new SDKError(
      `SDK requires circuits built for committee "${SDK_CIRCUIT_COMMITTEE}" ` +
        `(active preset is "${committee ?? 'unknown'}"). ` +
        `Run: pnpm -C packages/enclave-sdk compile:circuits`,
      'SDK_CIRCUIT_COMMITTEE_MISMATCH',
    )
  }

  checked = true
}
