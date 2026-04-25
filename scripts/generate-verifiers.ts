#!/usr/bin/env tsx
// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

/**
 * Generate Solidity verifier contracts from compiled Noir circuits.
 *
 * Prerequisites:
 *   - `nargo` and `bb` (Barretenberg CLI) must be installed and in PATH
 *   - Circuits should be compiled first (`pnpm build:circuits`) or this script
 *     will compile them automatically.
 *
 * Usage:
 *   pnpm generate:verifiers                  # All circuits (or --circuits from package.json)
 *   pnpm generate:verifiers --circuits pk,fold  # Specific circuits
 *   pnpm generate:verifiers --clean          # Remove existing verifiers first
 *   pnpm generate:verifiers --dry-run        # Show what would be generated
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
    console.log('🔮 Generating Solidity verifiers from Noir circuits...\n')

    this.checkTool('nargo --version', 'nargo')
    this.checkTool('bb --version', 'bb')

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

    // Clean stale nargo build caches to prevent using outdated artifacts
    this.cleanTargetDirs(circuits)

    // Prepare output directory
    if (this.options.clean && existsSync(this.verifierDir)) {
      rmSync(this.verifierDir, { recursive: true })
      console.log('   🗑️  Cleaned existing verifier directory')
    }
    mkdirSync(this.verifierDir, { recursive: true })

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

    const generated: string[] = []
    const errors: string[] = []

    for (const circuit of circuits) {
      try {
        const solFile = this.generateVerifier(circuit)
        generated.push(solFile)
      } catch (error: any) {
        errors.push(`${circuit.group}/${circuit.name}: ${error.message}`)
        console.error(`   ✗ ${circuit.group}/${circuit.name}: ${error.message}`)
      }
    }

    console.log(`\n✅ Generated ${generated.length} Solidity verifier(s) in:`)
    console.log(`   ${this.verifierDir}\n`)

    for (const f of generated) {
      console.log(`   • ${basename(f)}`)
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

  private generateVerifier(circuit: CircuitInfo): string {
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

    // 4. Post-process: rename contract, add license header, copy to output
    const contractName = this.toContractName(name)
    const outputFileName = `${contractName}.sol`
    const outputPath = join(this.verifierDir, outputFileName)

    let solidity = readFileSync(rawSolPath, 'utf-8')

    // Replace the default contract name (HonkVerifier) with our descriptive name
    solidity = solidity.replace(/contract\s+HonkVerifier/g, `contract ${contractName}`)

    // Replace license header – bb produces Apache-2.0 by default
    solidity = solidity.replace(/\/\/\s*SPDX-License-Identifier:[^\n]*\n(\/\/[^\n]*\n)*/, LICENSE_HEADER)

    writeFileSync(outputPath, solidity)

    // Clean up intermediate file
    rmSync(rawSolPath, { force: true })

    console.log(`   ✓ ${group}/${name} → ${outputFileName}`)
    return outputPath
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

Generates Solidity verifier contracts from compiled Noir circuits
and places them in packages/enclave-contracts/contracts/verifiers/bfv/honk/.

Options:
  --circuits <list>      Circuit names (comma-separated). When omitted, generates all circuits.
  --group <groups>       Circuit groups (comma-separated: dkg,threshold,recursive_aggregation)
  --clean                Remove existing verifier directory before generating
  --no-compile           Don't compile circuits automatically (fail if not already compiled)
  --dry-run              Show what would be generated without doing anything
  -h, --help             Show this help message

Examples:
  pnpm generate:verifiers --circuits dkg_aggregator,decryption_aggregator
  pnpm generate:verifiers --circuits dkg_aggregator --clean
`)
}

if (require.main === module) main()

export { VerifierGenerator, GenerateOptions, CircuitGroup, CIRCUIT_GROUPS }
