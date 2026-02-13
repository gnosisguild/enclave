#!/usr/bin/env tsx
// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { execSync } from 'child_process'
import { createHash } from 'crypto'
import { appendFileSync, copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'fs'
import { basename, join, resolve } from 'path'
import { ALL_GROUPS, CIRCUIT_GROUPS, type CircuitGroup } from './circuit-constants'

interface CircuitInfo {
  name: string
  group: CircuitGroup
  path: string
}
interface CompiledCircuit {
  name: string
  group: CircuitGroup
  artifacts: { json?: string; vk?: string }
  checksums: { json?: string; vk?: string }
}
interface BuildOptions {
  groups?: CircuitGroup[]
  circuits?: string[]
  skipChecksums?: boolean
  skipVk?: boolean
  outputDir?: string
  clean?: boolean
  dryRun?: boolean
}
interface BuildResult {
  success: boolean
  compiled: CompiledCircuit[]
  checksumFile?: string
  releaseDir?: string
  errors: string[]
  sourceHash?: string
}

class NoirCircuitBuilder {
  private rootDir: string
  private circuitsDir: string
  private options: BuildOptions

  constructor(rootDir?: string, options: BuildOptions = {}) {
    this.rootDir = rootDir ?? resolve(__dirname, '..')
    this.circuitsDir = join(this.rootDir, 'circuits', 'bin')
    this.options = {
      groups: ALL_GROUPS,
      outputDir: join(this.rootDir, 'dist', 'circuits'),
      clean: true,
      skipVk: false,
      ...options,
    }
  }

  async buildAll(): Promise<BuildResult> {
    const result: BuildResult = { success: true, compiled: [], errors: [] }

    console.log('🔮 Building Noir circuits...')

    try {
      this.checkTool('nargo --version', 'nargo')
      if (!this.options.skipVk) this.checkTool('bb --version', 'bb')

      const circuits = this.discoverCircuits()
      if (circuits.length === 0) {
        console.log('   ⚠️  No circuits found')
        return result
      }

      console.log(`   Found ${circuits.length} circuit(s)`)

      if (this.options.dryRun) {
        console.log('\n   Would build:', circuits.map((c) => `${c.group}/${c.name}`).join(', '))
        return result
      }

      if (this.options.clean && existsSync(this.options.outputDir!)) {
        rmSync(this.options.outputDir!, { recursive: true })
      }
      mkdirSync(this.options.outputDir!, { recursive: true })

      result.sourceHash = this.computeSourceHash()

      for (const circuit of circuits) {
        try {
          result.compiled.push(this.buildCircuit(circuit))
        } catch (error: any) {
          result.errors.push(`${circuit.name}: ${error.message}`)
          result.success = false
        }
      }

      if (!this.options.skipChecksums && result.compiled.length > 0) {
        result.checksumFile = this.generateChecksumFile(result.compiled)
      }

      result.releaseDir = this.copyArtifacts(result.compiled)
      console.log(`\n✅ Built ${result.compiled.length} circuits`)
    } catch (error: any) {
      result.success = false
      result.errors.push(error.message)
      console.error('❌ Error:', error.message)
    }

    return result
  }

  private checkTool(cmd: string, name: string): void {
    try {
      execSync(cmd, { stdio: ['pipe', 'pipe', 'pipe'] })
    } catch {
      throw new Error(`${name} is not installed or not in PATH`)
    }
  }

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
            circuits.push({ name: entry, group, path: circuitPath })
          }
        }
      }
    }
    return circuits
  }

  private buildCircuit(circuit: CircuitInfo): CompiledCircuit {
    const packageName = this.getPackageName(circuit.path)
    const result: CompiledCircuit = {
      name: circuit.name,
      group: circuit.group,
      artifacts: {},
      checksums: {},
    }

    execSync('nargo compile', { cwd: circuit.path, stdio: 'pipe' })

    const groupDir = join(this.circuitsDir, circuit.group)
    const targetDirs = [join(groupDir, 'target'), join(this.circuitsDir, 'target'), join(circuit.path, 'target')]

    let jsonFile: string | null = null
    let targetDir: string | null = null

    for (const dir of targetDirs) {
      if (!existsSync(dir)) continue
      const candidate = join(dir, `${packageName}.json`)
      if (existsSync(candidate)) {
        jsonFile = candidate
        targetDir = dir
        break
      }
    }

    if (!jsonFile || !targetDir) {
      throw new Error(
        `${circuit.group}/${circuit.name}: compiled artifact not found. ` + `Searched for ${packageName}.json in: ${targetDirs.join(', ')}`,
      )
    }

    this.sanitizePaths(jsonFile)
    result.artifacts.json = jsonFile
    result.checksums.json = this.checksum(jsonFile)

    if (!this.options.skipVk) {
      const vkFile = this.generateVk(jsonFile, targetDir, packageName)
      if (vkFile) {
        result.artifacts.vk = vkFile
        result.checksums.vk = this.checksum(vkFile)
      }
    }
    console.log(`   ✓ ${circuit.group}/${circuit.name}`)

    return result
  }

  private generateVk(jsonFile: string, targetDir: string, packageName: string): string | null {
    const vkFile = join(targetDir, `${packageName}.vk`)
    try {
      execSync(`bb write_vk -b "${jsonFile}" -o "${targetDir}" --oracle_hash keccak`, { stdio: 'pipe' })
      const defaultVk = join(targetDir, 'vk')
      if (existsSync(defaultVk)) {
        if (existsSync(vkFile)) rmSync(vkFile)
        copyFileSync(defaultVk, vkFile)
        rmSync(defaultVk)
      }
      return existsSync(vkFile) ? vkFile : null
    } catch (err) {
      console.error(`Error generating VK for ${jsonFile}:`, err)
      return null
    }
  }

  private getPackageName(circuitPath: string): string {
    try {
      const content = readFileSync(join(circuitPath, 'Nargo.toml'), 'utf-8')
      const match = content.match(/^name\s*=\s*"([^"]+)"/m)
      if (match) return match[1]
    } catch {
      // Ignore errors
    }
    return basename(circuitPath)
  }

  private sanitizePaths(jsonFile: string): void {
    try {
      const content = readFileSync(jsonFile, 'utf-8')
      const sanitized = content
        .replace(/"path"\s*:\s*"[^"]*[/\\](enclave[/\\]circuits[/\\][^"]+)"/g, '"path":"$1"')
        .replace(/"path"\s*:\s*"(?:\/[^"]*|[A-Za-z]:\\[^"]*)[/\\](circuits[/\\][^"]+)"/g, '"path":"enclave/$1"')
      if (content !== sanitized) writeFileSync(jsonFile, sanitized)
    } catch {
      // Ignore errors
    }
  }

  private checksum(filePath: string): string {
    return createHash('sha256').update(readFileSync(filePath)).digest('hex')
  }

  private generateChecksumFile(compiled: CompiledCircuit[]): string {
    const lines: string[] = []
    const checksums: Record<string, string> = {}

    for (const c of compiled) {
      const prefix = `${c.group}/${c.name}`
      if (c.checksums.json && c.artifacts.json) {
        const f = `${prefix}/${basename(c.artifacts.json)}`
        checksums[f] = c.checksums.json
        lines.push(`${c.checksums.json}  ${f}`)
      }
      if (c.checksums.vk && c.artifacts.vk) {
        const f = `${prefix}/${basename(c.artifacts.vk)}`
        checksums[f] = c.checksums.vk
        lines.push(`${c.checksums.vk}  ${f}`)
      }
    }

    const outputDir = this.options.outputDir!
    writeFileSync(join(outputDir, 'SHA256SUMS'), lines.join('\n') + '\n')
    writeFileSync(
      join(outputDir, 'checksums.json'),
      JSON.stringify({ algorithm: 'sha256', generated: new Date().toISOString(), files: checksums }, null, 2) + '\n',
    )
    return join(outputDir, 'SHA256SUMS')
  }

  private copyArtifacts(compiled: CompiledCircuit[]): string {
    const outputDir = this.options.outputDir!
    for (const c of compiled) {
      if (!c.artifacts.json && !c.artifacts.vk) continue
      const dir = join(outputDir, c.group, c.name)
      mkdirSync(dir, { recursive: true })
      if (c.artifacts.json) copyFileSync(c.artifacts.json, join(dir, basename(c.artifacts.json)))
      if (c.artifacts.vk) copyFileSync(c.artifacts.vk, join(dir, basename(c.artifacts.vk)))
    }
    return outputDir
  }

  computeSourceHash(): string {
    const hash = createHash('sha256')
    const circuits = this.discoverCircuits().sort((a, b) => `${a.group}/${a.name}`.localeCompare(`${b.group}/${b.name}`))
    for (const c of circuits) this.hashDir(c.path, hash)
    return hash.digest('hex').substring(0, 16)
  }

  private hashDir(dirPath: string, hash: ReturnType<typeof createHash>, relativePath = ''): void {
    for (const entry of readdirSync(dirPath).sort()) {
      if (entry === 'target' || entry.startsWith('.')) continue
      const fullPath = join(dirPath, entry)
      const entryRelativePath = relativePath ? `${relativePath}/${entry}` : entry
      const stat = statSync(fullPath)
      if (stat.isDirectory()) {
        hash.update(entryRelativePath + '/')
        this.hashDir(fullPath, hash, entryRelativePath)
      } else if (stat.isFile()) {
        hash.update(entryRelativePath)
        hash.update(readFileSync(fullPath))
      }
    }
  }

  writeGitHubOutput(result: BuildResult): void {
    const output = process.env.GITHUB_OUTPUT
    const lines = [
      `circuits_built=${result.compiled.length}`,
      `circuits_success=${result.success}`,
      `source_hash=${result.sourceHash ?? 'unknown'}`,
      result.releaseDir ? `artifacts_dir=${result.releaseDir}` : '',
    ].filter(Boolean)

    if (output) appendFileSync(output, lines.join('\n') + '\n')
    else console.log('\n📋 CI Output:', lines.join(', '))
  }
}

// CLI
async function main() {
  const args = process.argv.slice(2)
  const options: BuildOptions = {}
  let command = 'build'

  for (let i = 0; i < args.length; i++) {
    const arg = args[i]
    if (arg === '-h' || arg === '--help') {
      showHelp()
      process.exit(0)
    } else if (arg === '--dry-run') options.dryRun = true
    else if (arg === '--skip-checksums') options.skipChecksums = true
    else if (arg === '--skip-vk') options.skipVk = true
    else if (arg === '--no-clean') options.clean = false
    else if (arg === '--group') options.groups = args[++i]?.split(',') as CircuitGroup[]
    else if (arg === '--circuit') (options.circuits ??= []).push(args[++i])
    else if (arg === '-o' || arg === '--output') options.outputDir = resolve(args[++i])
    else if (['hash', 'build'].includes(arg)) command = arg
  }

  const builder = new NoirCircuitBuilder(undefined, options)

  if (command === 'hash') {
    const hash = builder.computeSourceHash()
    console.log(hash)
    if (process.env.GITHUB_OUTPUT) appendFileSync(process.env.GITHUB_OUTPUT, `source_hash=${hash}\n`)
  } else {
    const result = await builder.buildAll()
    builder.writeGitHubOutput(result)
    process.exit(result.success ? 0 : 1)
  }
}

function showHelp() {
  console.log(`
Usage: build-circuits [command] [options]

Commands: build (default), hash

Options:
  --group <groups>    Circuit groups (comma-separated: dkg,threshold)
  --circuit <name>    Build specific circuit(s)
  --skip-vk           Skip verification key generation
  --skip-checksums    Skip checksum generation
  -o, --output <dir>  Output directory (default: dist/circuits)
  --dry-run           Show what would be built
  --no-clean          Don't clean output directory
  -h, --help          Show help
`)
}

if (require.main === module) main()

export { NoirCircuitBuilder, BuildOptions, BuildResult, CompiledCircuit, CircuitInfo, CircuitGroup, CIRCUIT_GROUPS }
