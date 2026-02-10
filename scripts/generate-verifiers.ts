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
 *   pnpm generate:verifiers                  # Generate all verifiers
 *   pnpm generate:verifiers --group dkg      # Only DKG circuits
 *   pnpm generate:verifiers --group threshold # Only Threshold circuits
 *   pnpm generate:verifiers --circuit pk      # Only a specific circuit
 *   pnpm generate:verifiers --clean           # Remove existing verifiers first
 *   pnpm generate:verifiers --dry-run         # Show what would be generated
 */

import { execSync } from 'child_process'
import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'fs'
import { basename, join, resolve } from 'path'

// ---------------------------------------------------------------------------
// Types & constants
// ---------------------------------------------------------------------------

const CIRCUIT_GROUPS = {
  DKG: 'dkg',
  THRESHOLD: 'threshold',
  AGGREGATION: 'recursive_aggregation',
} as const

type CircuitGroup = (typeof CIRCUIT_GROUPS)[keyof typeof CIRCUIT_GROUPS]
const ALL_GROUPS: CircuitGroup[] = [CIRCUIT_GROUPS.DKG, CIRCUIT_GROUPS.THRESHOLD, CIRCUIT_GROUPS.AGGREGATION]

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
  oracleHash?: string // oracle hash scheme for bb write_vk (default: keccak)
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
    this.verifierDir = join(this.rootDir, 'packages', 'enclave-contracts', 'contracts', 'verifier')
    this.options = {
      groups: ALL_GROUPS,
      clean: false,
      compile: true,
      oracleHash: 'keccak',
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

    // Prepare output directory
    if (this.options.clean && existsSync(this.verifierDir)) {
      rmSync(this.verifierDir, { recursive: true })
      console.log('   🗑️  Cleaned existing verifier directory')
    }
    mkdirSync(this.verifierDir, { recursive: true })

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

    for (const group of this.options.groups ?? ALL_GROUPS) {
      const groupDir = join(this.circuitsDir, group)
      if (!existsSync(groupDir)) continue

      for (const entry of readdirSync(groupDir)) {
        const circuitPath = join(groupDir, entry)
        if (statSync(circuitPath).isDirectory() && existsSync(join(circuitPath, 'Nargo.toml'))) {
          if (!this.options.circuits || this.options.circuits.includes(entry)) {
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
    const contractName = this.toContractName(group, name)
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

    // Generate VK
    const oracleHashFlag = this.options.oracleHash ? ` --oracle_hash ${this.options.oracleHash}` : ''
    execSync(`bb write_vk -b "${jsonFile}" -o "${targetDir}"${oracleHashFlag}`, { stdio: 'pipe' })

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
   * Convert group/name to a PascalCase Solidity contract name.
   * e.g. (dkg, pk) → DkgPkVerifier
   *      (threshold, pk_generation) → ThresholdPkGenerationVerifier
   *      (recursive_aggregation, fold) → RecursiveAggregationFoldVerifier
   */
  private toContractName(group: CircuitGroup, name: string): string {
    const pascal = (s: string) =>
      s
        .split(/[_-]+/)
        .map((w) => w.charAt(0).toUpperCase() + w.slice(1).toLowerCase())
        .join('')
    return `${pascal(group)}${pascal(name)}Verifier`
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
      options.groups = args[++i]?.split(',') as CircuitGroup[]
    } else if (arg === '--circuit') {
      ;(options.circuits ??= []).push(args[++i])
    } else if (arg === '--oracle-hash') {
      options.oracleHash = args[++i]
    }
  }

  const generator = new VerifierGenerator(undefined, options)
  await generator.generate()
}

function showHelp() {
  console.log(`
Usage: generate-verifiers [options]

Generates Solidity verifier contracts from compiled Noir circuits
and places them in packages/enclave-contracts/contracts/verifier/.

Options:
  --group <groups>       Circuit groups (comma-separated: dkg,threshold,recursive_aggregation)
  --circuit <name>       Generate verifier for specific circuit(s) (repeatable)
  --clean                Remove existing verifier directory before generating
  --no-compile           Don't compile circuits automatically (fail if not already compiled)
  --oracle-hash <hash>   Oracle hash scheme for VK generation (default: keccak)
  --dry-run              Show what would be generated without doing anything
  -h, --help             Show this help message

Examples:
  pnpm generate:verifiers                          # All circuits
  pnpm generate:verifiers --group dkg               # Only DKG circuits
  pnpm generate:verifiers --group threshold --clean  # Threshold only, clean first
  pnpm generate:verifiers --circuit pk --circuit fold # Specific circuits
`)
}

if (require.main === module) main()

export { VerifierGenerator, GenerateOptions, CircuitGroup, CIRCUIT_GROUPS }
