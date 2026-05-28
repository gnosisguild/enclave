// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { existsSync, readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import { SDKError } from '../utils'

/** Matches `IEnclave.CommitteeSize.Micro` and `DEFAULT_E3_CONFIG.committeeSize`. */
export const SDK_CIRCUIT_COMMITTEE = 'micro'

function findActivePath(): string | null {
  // Walk up from the bundle file (depth varies: dist/ vs dist/crypto/ etc.)
  // until we find the package root (directory containing package.json).
  let dir = dirname(fileURLToPath(import.meta.url))
  while (true) {
    if (existsSync(resolve(dir, 'package.json'))) {
      // Bundled preset shipped inside the package takes priority (future use).
      const bundled = resolve(dir, '.active-preset.json')
      if (existsSync(bundled)) return bundled

      // When installed under node_modules the monorepo root is not available;
      // skip the check rather than resolving into an unrelated project tree.
      if (dir.includes('node_modules')) return null

      return resolve(dir, '../../circuits/bin/.active-preset.json')
    }
    const parent = dirname(dir)
    if (parent === dir) break
    dir = parent
  }
  throw new SDKError('Could not locate SDK package root', 'SDK_CIRCUIT_STAMP_MISSING')
}

const ACTIVE_PRESET_PATH = findActivePath()

let checked = false

/**
 * SDK encryption artifacts are built for the micro committee preset by default.
 * Fail fast when `circuits/bin/.active-preset.json` points at another committee
 * (e.g. after benchmark runs with `--committee small`).
 */
export function assertSdkMicroCircuits(): void {
  if (checked) return
  checked = true

  if (ACTIVE_PRESET_PATH === null) return

  let raw: string
  try {
    raw = readFileSync(ACTIVE_PRESET_PATH, 'utf-8')
  } catch {
    throw new SDKError(
      `Missing ${ACTIVE_PRESET_PATH}. Run \`pnpm -C packages/enclave-sdk compile:circuits\` first.`,
      'SDK_CIRCUIT_STAMP_MISSING',
    )
  }

  let committee: string | undefined
  try {
    committee = JSON.parse(raw)?.committee as string | undefined
  } catch {
    throw new SDKError(
      `Invalid JSON in ${ACTIVE_PRESET_PATH}. Rebuild with \`pnpm -C packages/enclave-sdk compile:circuits\`.`,
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
}
