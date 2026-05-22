#!/usr/bin/env tsx
// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

/**
 * Generate (or verify) Solidity verifier contracts from compiled Noir circuits.
 *
 * The Honk Solidity verifiers are committed to git. To keep the committed
 * files in sync with the recursive VKs, this script has two modes:
 *
 *   - `--check` (default for test/benchmark flows): regenerate in memory and
 *     diff against the committed files. Exits non-zero on drift without
 *     touching the working tree. Used by `prebuild.sh`, `extract_crisp_verify_gas.sh`,
 *     `replay_folded_verify_gas.sh` so accidental drift fails loudly instead
 *     of silently rewriting committed contracts mid-test.
 *
 *   - `--write` (default for manual runs): regenerate and overwrite the
 *     committed `.sol` files. Use this when you intentionally bump circuits.
 *
 * Prerequisites:
 *   - `nargo` and `bb` (Barretenberg CLI) must be installed and in PATH
 *   - Circuits should be compiled first (`pnpm build:circuits`) or this script
 *     will compile them automatically.
 *
 * Usage:
 *   pnpm generate:verifiers                       # Write all circuits (default)
 *   pnpm generate:verifiers --check               # Verify committed verifiers match VKs (no writes)
 *   pnpm generate:verifiers --circuits pk,fold    # Specific circuits
 *   pnpm generate:verifiers --clean               # Remove existing verifiers first (write mode only)
 *   pnpm generate:verifiers --dry-run             # Show what would be generated
 *   pnpm generate:verifiers --no-compile          # Use artifacts from build:circuits (skips target cleanup)
 */

import { execSync } from 'child_process'
import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'fs'
import { basename, join, resolve } from 'path'
import { ALL_GROUPS, CIRCUIT_GROUPS, type CircuitGroup } from './circuit-constants'

// ---------------------------------------------------------------------------
// Types & constants
// ---------------------------------------------------------------------------

const LICENSE_HEADER = `// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
`

/**
 * Canonical BFV preset for the committed Honk Solidity verifiers.
 *
 * The on-chain `DkgAggregatorVerifier.sol` / `DecryptionAggregatorVerifier.sol` bake in the
 * recursive VKs of `dkg_aggregator` / `decryption_aggregator`, which are
 * **preset-dependent**: different BFV parameter sets compile to different VKs and therefore
 * different `.sol` bytes. Exactly one preset can be "the committed one"; we pin `insecure-512`
 * because that is the development/CI/benchmark default and the preset every committed verifier
 * corresponds to.
 *
 * Both `--check` and `--write` refuse to run unless
 * `dist/circuits/<CANONICAL_PRESET>/.build-stamp.json` exists and its `preset` field matches.
 * This prevents silently producing/checking against the wrong preset's VKs — e.g. after
 * `pnpm build:circuits --preset secure-8192`, where `circuits/bin/` holds secure artifacts that
 * would generate different `.sol` bytes.
 *
 * If you need verifiers for a different preset (e.g. a production deploy on `secure-8192`),
 * rebuild that preset locally and run the generator there; do **not** commit the result over
 * the canonical files.
 */
const CANONICAL_PRESET = 'insecure-512'

interface CircuitInfo {
  name: string
  group: CircuitGroup
  path: string
  packageName: string
}

interface GenerateOptions {
  groups?: CircuitGroup[]
  circuits?: string[]
  clean?: boolean
  dryRun?: boolean
  compile?: boolean // compile circuits before generating verifiers
  noCleanTargets?: boolean // skip deleting nargo target dirs before generation
  /**
   * Check mode: generate verifiers into memory and diff against the committed
   * files. Exit non-zero on any drift. No writes to the working tree.
   * Used by test/benchmark/CI flows that must not silently mutate committed
   * verifier contracts.
   */
  check?: boolean
}

// ---------------------------------------------------------------------------
// Main class
// ---------------------------------------------------------------------------

class VerifierGenerator {
  private rootDir: string
  private circuitsDir: string
  private verifierDir: string
  private options: GenerateOptions

  constructor(rootDir?: string, options: GenerateOptions = {}) {
    this.rootDir = rootDir ?? resolve(__dirname, '..')
    this.circuitsDir = join(this.rootDir, 'circuits', 'bin')
    this.verifierDir = join(this.rootDir, 'packages', 'enclave-contracts', 'contracts', 'verifiers', 'bfv', 'honk')
    this.options = {
      groups: ALL_GROUPS,
      clean: false,
      compile: true,
      ...options,
    }
  }

  async generate(): Promise<void> {
    const mode = this.options.check ? 'Checking' : 'Generating'
    console.log(`🔮 ${mode} Solidity verifiers from Noir circuits...\n`)

    this.checkTool('nargo --version', 'nargo')
    this.checkTool('bb --version', 'bb')

    // Refuse to run unless `circuits/bin/` was last built for the canonical preset.
    // This is the only on-disk witness of "which preset's VKs live under circuits/bin"
    // (cf. `scripts/build-circuits.ts:writePresetStamp`). Skip the gate in --dry-run mode
    // (no artifact reads will happen) and in --no-compile mode only if explicitly allowed
    // by a future opt-out — today, even --no-compile must run against the canonical preset.
    if (!this.options.dryRun) {
      this.assertCanonicalPresetBuilt()
    }

    const circuits = this.discoverCircuits()
    if (circuits.length === 0) {
      console.log('   ⚠️  No circuits found')
      return
    }

    console.log(`   Found ${circuits.length} circuit(s)\n`)

    if (this.options.dryRun) {
      console.log('   Would generate verifiers for:', circuits.map((c) => `${c.group}/${c.name}`).join(', '))
      console.log(`   Output directory: ${this.verifierDir}`)
      return
    }

    // Clean stale nargo build caches unless caller just ran build:circuits (--no-compile / --no-clean-targets).
    if (!this.options.noCleanTargets && this.options.compile !== false) {
      this.cleanTargetDirs(circuits)
    }

    if (this.options.check && this.options.clean) {
      throw new Error('--check and --clean are mutually exclusive (check must not mutate the working tree)')
    }

    // In write mode, prepare output directory.
    if (!this.options.check) {
      if (this.options.clean && existsSync(this.verifierDir)) {
        rmSync(this.verifierDir, { recursive: true })
        console.log('   🗑️  Cleaned existing verifier directory')
      }
      mkdirSync(this.verifierDir, { recursive: true })
    } else if (!existsSync(this.verifierDir)) {
      throw new Error(`--check requires the committed verifier directory to exist: ${this.verifierDir}`)
    }

    // Pre-flight: two circuits with the same leaf name would silently overwrite each other's .sol.
    const seen = new Map<string, string>()
    for (const circuit of circuits) {
      const contractFile = `${this.toContractName(circuit.name)}.sol`
      const prior = seen.get(contractFile)
      if (prior) {
        throw new Error(
          `Duplicate Solidity verifier filename ${contractFile} for circuits ` +
            `${prior} and ${circuit.group}/${circuit.name}; rename one of the circuits or ` +
            `extend toContractName to include the group prefix.`,
        )
      }
      seen.set(contractFile, `${circuit.group}/${circuit.name}`)
    }

    const processed: string[] = []
    const drift: { circuit: string; file: string; reason: string }[] = []
    const errors: string[] = []

    for (const circuit of circuits) {
      try {
        const result = this.generateVerifier(circuit)
        processed.push(result.outputPath)
        if (this.options.check) {
          if (!existsSync(result.outputPath)) {
            drift.push({
              circuit: `${circuit.group}/${circuit.name}`,
              file: result.outputPath,
              reason: 'committed file is missing',
            })
          } else {
            const committed = readFileSync(result.outputPath, 'utf-8')
            if (committed !== result.content) {
              drift.push({
                circuit: `${circuit.group}/${circuit.name}`,
                file: result.outputPath,
                reason: 'content differs from committed file',
              })
            }
          }
        }
      } catch (error: any) {
        errors.push(`${circuit.group}/${circuit.name}: ${error.message}`)
        console.error(`   ✗ ${circuit.group}/${circuit.name}: ${error.message}`)
      }
    }

    if (this.options.check) {
      if (drift.length > 0) {
        console.error(`\n❌ ${drift.length} Solidity verifier(s) drift from current circuit VKs:`)
        for (const d of drift) {
          console.error(`   • ${d.circuit} (${basename(d.file)}): ${d.reason}`)
        }
        console.error(
          `\n   The committed Honk verifiers under contracts/verifiers/bfv/honk are out of sync\n` +
            `   with the circuits' recursive VKs. This usually means:\n` +
            `     - You ran 'pnpm build:circuits' against a different Noir/bb version, or\n` +
            `     - A circuit / VK changed without regenerating the committed Solidity files.\n` +
            `\n   To fix:\n` +
            `     1. Verify your Noir/bb versions match (see crates/zk-prover/versions.json).\n` +
            `     2. Run 'pnpm build:circuits --preset insecure-512' (or the relevant preset).\n` +
            `     3. Run 'pnpm generate:verifiers --write' (or omit --check) to refresh the\n` +
            `        committed .sol files, then commit the diff.\n`,
        )
        process.exit(1)
      }
      console.log(`\n✅ Checked ${processed.length} Solidity verifier(s) — all in sync with current VKs.\n`)
      for (const f of processed) console.log(`   • ${basename(f)}`)
    } else {
      console.log(`\n✅ Generated ${processed.length} Solidity verifier(s) in:`)
      console.log(`   ${this.verifierDir}\n`)
      for (const f of processed) console.log(`   • ${basename(f)}`)
    }

    if (errors.length > 0) {
      console.error(`\n❌ ${errors.length} error(s):`)
      for (const e of errors) console.error(`   • ${e}`)
      process.exit(1)
    }
  }

  // -------------------------------------------------------------------------
  // Discovery
  // -------------------------------------------------------------------------

  private discoverCircuits(): CircuitInfo[] {
    const circuits: CircuitInfo[] = []
    if (!existsSync(this.circuitsDir)) return circuits

    // When --circuits not specified, include all circuits
    const circuitFilter = this.options.circuits?.length ? this.options.circuits : undefined

    for (const group of this.options.groups ?? ALL_GROUPS) {
      const groupDir = join(this.circuitsDir, group)
      if (!existsSync(groupDir)) continue

      for (const entry of readdirSync(groupDir)) {
        const circuitPath = join(groupDir, entry)
        if (statSync(circuitPath).isDirectory() && existsSync(join(circuitPath, 'Nargo.toml'))) {
          if (!circuitFilter || circuitFilter.includes(entry)) {
            const packageName = this.getPackageName(circuitPath)
            circuits.push({ name: entry, group, path: circuitPath, packageName })
          }
        }
      }
    }
    return circuits
  }

  private getPackageName(circuitPath: string): string {
    try {
      const content = readFileSync(join(circuitPath, 'Nargo.toml'), 'utf-8')
      const match = content.match(/^name\s*=\s*"([^"]+)"/m)
      if (match) return match[1]
    } catch {
      // fall through
    }
    return basename(circuitPath)
  }

  // -------------------------------------------------------------------------
  // Generation pipeline  (compile → write_vk → write_solidity_verifier)
  // -------------------------------------------------------------------------

  private generateVerifier(circuit: CircuitInfo): { content: string; outputPath: string } {
    const { name, group, packageName } = circuit

    // 1. Compile if needed
    const jsonFile = this.ensureCompiled(circuit)

    // 2. Generate VK
    const targetDir = this.findTargetDir(circuit, packageName)
    const vkPath = this.ensureVk(jsonFile, targetDir, packageName)

    // 3. Generate Solidity verifier
    const rawSolPath = join(targetDir, `${packageName}_verifier.sol`)
    execSync(`bb write_solidity_verifier -k "${vkPath}" -o "${rawSolPath}"`, { stdio: 'pipe' })

    if (!existsSync(rawSolPath)) {
      throw new Error('bb write_solidity_verifier did not produce output')
    }

    // 4. Post-process: rename contract, add license header
    const contractName = this.toContractName(name)
    const outputFileName = `${contractName}.sol`
    const outputPath = join(this.verifierDir, outputFileName)

    let solidity = readFileSync(rawSolPath, 'utf-8')

    // Replace the default contract name (HonkVerifier) with our descriptive name
    solidity = solidity.replace(/contract\s+HonkVerifier/g, `contract ${contractName}`)

    // Replace license header – bb produces Apache-2.0 by default
    solidity = solidity.replace(/\/\/\s*SPDX-License-Identifier:[^\n]*\n(\/\/[^\n]*\n)*/, LICENSE_HEADER)

    // Clean up intermediate file (always — we don't keep the bb temp output around)
    rmSync(rawSolPath, { force: true })

    // Normalize with prettier so the on-disk format matches what the rest of
    // the repo's `pnpm prettier:write` produces. Without this, --check would
    // always fail because bb emits a different whitespace style than
    // prettier-plugin-solidity.
    solidity = this.formatSolidity(solidity, outputPath)

    // In check mode, do not touch the committed file.
    if (!this.options.check) {
      writeFileSync(outputPath, solidity)
      console.log(`   ✓ ${group}/${name} → ${outputFileName}`)
    } else {
      console.log(`   • ${group}/${name} → ${outputFileName} (checking)`)
    }

    return { content: solidity, outputPath }
  }

  /**
   * Format Solidity through prettier-plugin-solidity so output matches the
   * project's standard formatting. Run from `packages/enclave-contracts` so
   * prettier picks up the local `.prettierrc` and plugin resolution.
   */
  private formatSolidity(content: string, outputPath: string): string {
    const contractsDir = join(this.rootDir, 'packages', 'enclave-contracts')
    // Use prettier --stdin-filepath so plugin selection is by extension.
    const result = execSync(`pnpm exec prettier --stdin-filepath "${outputPath}"`, {
      cwd: contractsDir,
      input: content,
      stdio: ['pipe', 'pipe', 'pipe'],
    })
    return result.toString('utf-8')
  }

  /**
   * Ensure the circuit is compiled and return the path to the JSON artifact.
   */
  private ensureCompiled(circuit: CircuitInfo): string {
    const targetDirs = this.candidateTargetDirs(circuit)

    // Check if already compiled
    for (const dir of targetDirs) {
      const candidate = join(dir, `${circuit.packageName}.json`)
      if (existsSync(candidate)) return candidate
    }

    // Compile
    if (!this.options.compile) {
      throw new Error(`Compiled artifact ${circuit.packageName}.json not found. Run 'pnpm build:circuits' first or remove --no-compile.`)
    }

    console.log(`   ⏳ Compiling ${circuit.group}/${circuit.name}...`)
    execSync('nargo compile', { cwd: circuit.path, stdio: 'pipe' })

    // Search again
    for (const dir of targetDirs) {
      const candidate = join(dir, `${circuit.packageName}.json`)
      if (existsSync(candidate)) return candidate
    }

    throw new Error(`Compiled artifact ${circuit.packageName}.json not found after compilation`)
  }

  /**
   * Ensure VK exists and return its path.
   */
  private ensureVk(jsonFile: string, targetDir: string, packageName: string): string {
    const vkFile = join(targetDir, `${packageName}.vk`)

    // Check if VK already exists
    if (existsSync(vkFile)) return vkFile

    // Also check for a bare 'vk' file
    const defaultVk = join(targetDir, 'vk')
    if (existsSync(defaultVk)) {
      copyFileSync(defaultVk, vkFile)
      return vkFile
    }

    // Generate VK (EVM target for Solidity verifiers)
    execSync(`bb write_vk -b "${jsonFile}" -o "${targetDir}" -t evm`, { stdio: 'pipe' })

    // bb writes to 'vk' by default, rename to <packageName>.vk
    if (existsSync(defaultVk) && !existsSync(vkFile)) {
      copyFileSync(defaultVk, vkFile)
      rmSync(defaultVk, { force: true })
    }

    if (!existsSync(vkFile)) {
      throw new Error(`Failed to generate verification key for ${packageName}`)
    }

    return vkFile
  }

  // -------------------------------------------------------------------------
  // Helpers
  // -------------------------------------------------------------------------

  private candidateTargetDirs(circuit: CircuitInfo): string[] {
    const groupDir = join(this.circuitsDir, circuit.group)
    return [join(groupDir, 'target'), join(this.circuitsDir, 'target'), join(circuit.path, 'target')]
  }

  private findTargetDir(circuit: CircuitInfo, packageName: string): string {
    for (const dir of this.candidateTargetDirs(circuit)) {
      const candidate = join(dir, `${packageName}.json`)
      if (existsSync(candidate)) return dir
    }
    throw new Error(`Target directory not found for ${circuit.group}/${circuit.name}`)
  }

  /**
   * Convert circuit folder name to a PascalCase Solidity contract name.
   * e.g. dkg_aggregator → DkgAggregatorVerifier, decryption_aggregator → DecryptionAggregatorVerifier
   */
  private toContractName(name: string): string {
    const pascal = (s: string) =>
      s
        .split(/[_-]+/)
        .map((w) => w.charAt(0).toUpperCase() + w.slice(1).toLowerCase())
        .join('')
    return `${pascal(name)}Verifier`
  }

  /**
   * Remove all nargo target directories to prevent stale cached artifacts
   * from being picked up instead of freshly compiled ones.
   */
  private cleanTargetDirs(circuits: CircuitInfo[]): void {
    const cleaned = new Set<string>()
    for (const circuit of circuits) {
      const groupTarget = join(this.circuitsDir, circuit.group, 'target')
      if (!cleaned.has(groupTarget) && existsSync(groupTarget)) {
        rmSync(groupTarget, { recursive: true })
        cleaned.add(groupTarget)
      }
      const circuitTarget = join(circuit.path, 'target')
      if (!cleaned.has(circuitTarget) && existsSync(circuitTarget)) {
        rmSync(circuitTarget, { recursive: true })
        cleaned.add(circuitTarget)
      }
    }
    const rootTarget = join(this.circuitsDir, 'target')
    if (existsSync(rootTarget)) {
      rmSync(rootTarget, { recursive: true })
      cleaned.add(rootTarget)
    }
    if (cleaned.size > 0) {
      console.log(`   🧹 Cleaned ${cleaned.size} stale target dir(s)`)
    }
  }

  private checkTool(cmd: string, name: string): void {
    try {
      execSync(cmd, { stdio: ['pipe', 'pipe', 'pipe'] })
    } catch {
      throw new Error(`${name} is not installed or not in PATH`)
    }
  }

  /**
   * Refuse to run unless `circuits/bin/` was last built for `CANONICAL_PRESET`.
   *
   * The committed Honk Solidity verifiers correspond to one preset
   * (`CANONICAL_PRESET`). The only on-disk record of "which preset built
   * `circuits/bin/`" is the build stamp written by
   * `scripts/build-circuits.ts:writePresetStamp` at
   * `dist/circuits/<preset>/.build-stamp.json`. If the stamp for `CANONICAL_PRESET`
   * is missing, the developer either never built it, or last built a different
   * preset — either way, generating/checking against `circuits/bin/` would use
   * the wrong VKs. Surface that loudly with a fix recipe instead of silently
   * producing wrong verifier bytes.
   */
  private assertCanonicalPresetBuilt(): void {
    const stampPath = join(this.rootDir, 'dist', 'circuits', CANONICAL_PRESET, '.build-stamp.json')
    if (!existsSync(stampPath)) {
      throw new Error(
        `Canonical preset '${CANONICAL_PRESET}' is not built (missing ${stampPath}).\n` +
          `   The committed Solidity Honk verifiers under\n` +
          `   packages/enclave-contracts/contracts/verifiers/bfv/honk/ bake in the recursive VKs\n` +
          `   of the '${CANONICAL_PRESET}' BFV preset. Generating/checking against any other preset\n` +
          `   would produce different '.sol' bytes and is rejected by design.\n` +
          `\n` +
          `   To fix, run from the repo root:\n` +
          `     pnpm build:circuits --preset ${CANONICAL_PRESET}\n` +
          `   then retry. If you intentionally want verifiers for a different preset (e.g. a\n` +
          `   production deploy on 'secure-8192'), generate them locally for that deploy — do\n` +
          `   NOT commit the result over the canonical files.`,
      )
    }
    let stamp: { preset?: string } = {}
    try {
      stamp = JSON.parse(readFileSync(stampPath, 'utf-8'))
    } catch (err: any) {
      throw new Error(`Failed to parse ${stampPath}: ${err.message}`)
    }
    if (stamp.preset !== CANONICAL_PRESET) {
      throw new Error(
        `Build stamp at ${stampPath} reports preset '${stamp.preset ?? '(missing)'}', expected '${CANONICAL_PRESET}'.\n` +
          `   The committed Solidity Honk verifiers correspond to '${CANONICAL_PRESET}' only.\n` +
          `   Run:\n` +
          `     pnpm build:circuits --preset ${CANONICAL_PRESET}\n` +
          `   then retry.`,
      )
    }
    console.log(`   ✓ Canonical preset '${CANONICAL_PRESET}' build stamp present.\n`)
  }
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

async function main() {
  const args = process.argv.slice(2)
  const options: GenerateOptions = {}

  for (let i = 0; i < args.length; i++) {
    const arg = args[i]
    if (arg === '-h' || arg === '--help') {
      showHelp()
      process.exit(0)
    } else if (arg === '--dry-run') {
      options.dryRun = true
    } else if (arg === '--clean') {
      options.clean = true
    } else if (arg === '--no-compile') {
      options.compile = false
      options.noCleanTargets = true
    } else if (arg === '--no-clean-targets') {
      options.noCleanTargets = true
    } else if (arg === '--check') {
      options.check = true
    } else if (arg === '--write') {
      options.check = false
    } else if (arg === '--group') {
      const value = args[++i]
      if (!value || value.startsWith('--')) {
        console.error('Error: --group requires a value')
        process.exit(1)
      }
      options.groups = value.split(',') as CircuitGroup[]
    } else if (arg === '--circuits') {
      const value = args[++i]
      if (!value || value.startsWith('--')) {
        console.error('Error: --circuits requires a value')
        process.exit(1)
      }
      options.circuits = value.split(',').map((s) => s.trim())
    }
  }

  const generator = new VerifierGenerator(undefined, options)
  await generator.generate()
}

function showHelp() {
  console.log(`
Usage: generate-verifiers [options]

Generates (or verifies) Solidity Honk verifier contracts from compiled Noir
circuits and places them in packages/enclave-contracts/contracts/verifiers/bfv/honk/.

The Solidity verifiers are committed to git. Test and benchmark flows run
this script with --check so accidental drift between committed verifiers and
current circuit VKs is surfaced as a failure rather than a silent rewrite.

Options:
  --check                Verify committed verifiers match current VKs (no writes).
                         Exits non-zero on drift. Used by test/benchmark/CI flows.
  --write                Write/overwrite committed verifiers (this is the default
                         when neither --check nor --write is passed).
  --circuits <list>      Circuit names (comma-separated). When omitted, generates all circuits.
  --group <groups>       Circuit groups (comma-separated: dkg,threshold,recursive_aggregation)
  --clean                Remove existing verifier directory before generating (write mode only).
  --no-compile           Don't compile circuits automatically (fail if not already compiled);
                         also skips cleaning nargo target dirs (use after build:circuits).
  --no-clean-targets     Don't delete nargo target dirs before generating verifiers.
  --dry-run              Show what would be generated without doing anything.
  -h, --help             Show this help message.

Examples:
  pnpm generate:verifiers                                # Rewrite committed verifiers (manual)
  pnpm generate:verifiers --check                        # Verify committed verifiers (CI/tests)
  pnpm generate:verifiers --circuits dkg_aggregator      # Single circuit
  pnpm generate:verifiers --check --no-compile           # Verify against existing artifacts only
`)
}

if (require.main === module) {
  main().catch((err: unknown) => {
    const msg = err instanceof Error ? err.message : String(err)
    console.error(`\n❌ ${msg}\n`)
    process.exit(1)
  })
}

export { VerifierGenerator, GenerateOptions, CircuitGroup, CIRCUIT_GROUPS }
