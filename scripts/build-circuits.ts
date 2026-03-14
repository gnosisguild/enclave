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
import { ALL_GROUPS, CIRCUIT_GROUPS, CIRCUIT_VARIANTS, type CircuitGroup } from './circuit-constants'

/** share_computation wrapper is shared by sk_share_computation and e_sm_share_computation. */
const SHARE_COMP_WRAPPER = {
  path: ['recursive_aggregation', 'wrapper', 'dkg'] as const,
  aliases: ['sk_share_computation', 'e_sm_share_computation'] as const,
  variants: [CIRCUIT_VARIANTS.DEFAULT, CIRCUIT_VARIANTS.RECURSIVE] as const,
}

interface CircuitInfo {
  name: string
  group: CircuitGroup
  path: string
}
interface CompiledCircuit {
  name: string
  group: CircuitGroup
  artifacts: {
    json?: string
    vk?: string
    vkHash?: string
    vkRecursive?: string
    vkRecursiveHash?: string
    vkNoir?: string
    vkNoirHash?: string
  }
  checksums: {
    json?: string
    vk?: string
    vkHash?: string
    vkRecursive?: string
    vkRecursiveHash?: string
    vkNoir?: string
    vkNoirHash?: string
  }
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

      if (this.options.clean) {
        if (existsSync(this.options.outputDir!)) {
          rmSync(this.options.outputDir!, { recursive: true })
        }
        this.cleanTargetDirs(circuits)
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
      if (result.errors.length > 0) {
        console.error('\n❌ Failed circuits:')
        for (const err of result.errors) console.error(`   ${err}`)
      }
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

  private cleanTargetDirs(circuits: CircuitInfo[]): void {
    const cleaned = new Set<string>()
    for (const circuit of circuits) {
      // Clean group-level target (e.g. circuits/bin/dkg/target)
      const groupTarget = join(this.circuitsDir, circuit.group, 'target')
      if (!cleaned.has(groupTarget) && existsSync(groupTarget)) {
        rmSync(groupTarget, { recursive: true })
        cleaned.add(groupTarget)
      }
      // Clean circuit-level target (e.g. circuits/bin/dkg/pk/target)
      const circuitTarget = join(circuit.path, 'target')
      if (!cleaned.has(circuitTarget) && existsSync(circuitTarget)) {
        rmSync(circuitTarget, { recursive: true })
        cleaned.add(circuitTarget)
      }
    }
    // Clean root-level target (circuits/bin/target)
    const rootTarget = join(this.circuitsDir, 'target')
    if (existsSync(rootTarget)) {
      rmSync(rootTarget, { recursive: true })
      cleaned.add(rootTarget)
    }
    if (cleaned.size > 0) {
      console.log(`   🧹 Cleaned ${cleaned.size} stale target dir(s)`)
    }
  }

  private discoverCircuits(): CircuitInfo[] {
    const circuits: CircuitInfo[] = []
    if (!existsSync(this.circuitsDir)) return circuits

    for (const group of this.options.groups ?? ALL_GROUPS) {
      const groupDir = join(this.circuitsDir, group)
      if (!existsSync(groupDir)) continue

      this.findCircuitsInDir(groupDir, '', group, circuits)
    }
    return circuits
  }

  private findCircuitsInDir(dir: string, relativePath: string, group: CircuitGroup, out: CircuitInfo[]): void {
    for (const entry of readdirSync(dir)) {
      const fullPath = join(dir, entry)
      if (!statSync(fullPath).isDirectory()) continue

      const name = relativePath ? `${relativePath}/${entry}` : entry
      const nargoPath = join(fullPath, 'Nargo.toml')
      if (!existsSync(nargoPath)) {
        this.findCircuitsInDir(fullPath, name, group, out)
        continue
      }
      // Workspace roots ([workspace]) are not circuits; recurse to find leaf packages
      if (this.isWorkspaceOnly(nargoPath)) {
        this.findCircuitsInDir(fullPath, name, group, out)
      } else if (!this.options.circuits || this.options.circuits.includes(name)) {
        out.push({ name, group, path: fullPath })
      }
    }
  }

  private isWorkspaceOnly(nargoPath: string): boolean {
    const content = readFileSync(nargoPath, 'utf-8')
    return /^\s*\[workspace\]/m.test(content) && !/^\s*\[package\]/m.test(content)
  }

  /** Search dirs for compiled JSON; include parent targets (workspace members output to workspace root). */
  private getTargetSearchDirs(circuitPath: string, groupDir: string): string[] {
    const dirs = [join(circuitPath, 'target')]
    let dir = circuitPath
    while (dir !== groupDir) {
      const parent = resolve(dir, '..')
      if (parent === dir) break
      dirs.push(join(parent, 'target'))
      dir = parent
    }
    dirs.push(join(groupDir, 'target'), join(this.circuitsDir, 'target'))
    return dirs
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
    const targetDirs = this.getTargetSearchDirs(circuit.path, groupDir)

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
      const vkArtifacts = this.generateVk(circuit, jsonFile, targetDir, packageName)
      if (vkArtifacts.vk) {
        result.artifacts.vk = vkArtifacts.vk
        result.checksums.vk = this.checksum(vkArtifacts.vk)
      }
      if (vkArtifacts.vkHash && existsSync(vkArtifacts.vkHash)) {
        result.artifacts.vkHash = vkArtifacts.vkHash
        result.checksums.vkHash = this.checksum(vkArtifacts.vkHash)
      }
      if (vkArtifacts.vkRecursive) {
        result.artifacts.vkRecursive = vkArtifacts.vkRecursive
        result.checksums.vkRecursive = this.checksum(vkArtifacts.vkRecursive)
      }
      if (vkArtifacts.vkRecursiveHash && existsSync(vkArtifacts.vkRecursiveHash)) {
        result.artifacts.vkRecursiveHash = vkArtifacts.vkRecursiveHash
        result.checksums.vkRecursiveHash = this.checksum(vkArtifacts.vkRecursiveHash)
      }
      if (vkArtifacts.vkNoir) {
        result.artifacts.vkNoir = vkArtifacts.vkNoir
        result.checksums.vkNoir = this.checksum(vkArtifacts.vkNoir)
      }
      if (vkArtifacts.vkNoirHash && existsSync(vkArtifacts.vkNoirHash)) {
        result.artifacts.vkNoirHash = vkArtifacts.vkNoirHash
        result.checksums.vkNoirHash = this.checksum(vkArtifacts.vkNoirHash)
      }
    }
    console.log(`   ✓ ${circuit.group}/${circuit.name}`)

    return result
  }

  private isWrapper(circuit: CircuitInfo): boolean {
    return circuit.name.startsWith('wrapper/')
  }

  private generateVk(
    circuit: CircuitInfo,
    jsonFile: string,
    targetDir: string,
    packageName: string,
  ): {
    vk: string | null
    vkHash: string | null
    vkRecursive: string | null
    vkRecursiveHash: string | null
    vkNoir: string | null
    vkNoirHash: string | null
  } {
    const result = {
      vk: null as string | null,
      vkHash: null as string | null,
      vkRecursive: null as string | null,
      vkRecursiveHash: null as string | null,
      vkNoir: null as string | null,
      vkNoirHash: null as string | null,
    }
    const isWrapper = this.isWrapper(circuit)

    const runWriteVk = (verifierTarget: string, vkOut: string, vkHashOut: string): boolean => {
      try {
        execSync(`bb write_vk -b "${jsonFile}" -o "${targetDir}" -t ${verifierTarget}`, { stdio: 'pipe' })
        const defaultVk = join(targetDir, 'vk')
        const defaultVkHash = join(targetDir, 'vk_hash')
        if (!existsSync(defaultVk) || !existsSync(defaultVkHash)) {
          console.error(
            `VK artifacts missing after bb write_vk (${verifierTarget}) for ${jsonFile}: expected ${defaultVk} and ${defaultVkHash}`,
          )
          return false
        }
        if (existsSync(vkOut)) rmSync(vkOut)
        copyFileSync(defaultVk, vkOut)
        rmSync(defaultVk)
        if (existsSync(vkHashOut)) rmSync(vkHashOut)
        copyFileSync(defaultVkHash, vkHashOut)
        rmSync(defaultVkHash)
        return true
      } catch (err) {
        console.error(`Error generating VK (${verifierTarget}) for ${jsonFile}:`, err)
        return false
      }
    }

    const vkFile = join(targetDir, `${packageName}.vk`)
    const vkHashFile = join(targetDir, `${packageName}.vk_hash`)
    const vkRecursiveFile = join(targetDir, `${packageName}.vk_recursive`)
    const vkRecursiveHashFile = join(targetDir, `${packageName}.vk_recursive_hash`)
    const vkNoirFile = join(targetDir, `${packageName}.vk_noir`)
    const vkNoirHashFile = join(targetDir, `${packageName}.vk_noir_hash`)

    if (!isWrapper) {
      // evm VK: for on-chain Solidity verification
      if (!runWriteVk('evm', vkFile, vkHashFile)) {
        throw new Error(`VK generation failed for ${packageName} (evm)`)
      }
      result.vk = existsSync(vkFile) ? vkFile : null
      result.vkHash = existsSync(vkHashFile) ? vkHashFile : null

      // noir-recursive-no-zk VK: for wrapper/fold output verification (Default variant)
      if (!runWriteVk('noir-recursive-no-zk', vkRecursiveFile, vkRecursiveHashFile)) {
        throw new Error(`VK generation failed for ${packageName} (noir-recursive-no-zk)`)
      }
      result.vkRecursive = existsSync(vkRecursiveFile) ? vkRecursiveFile : null
      result.vkRecursiveHash = existsSync(vkRecursiveHashFile) ? vkRecursiveHashFile : null

      // noir-recursive VK: for inner/base proofs embedded in wrapper inputs (Recursive variant)
      if (!runWriteVk('noir-recursive', vkNoirFile, vkNoirHashFile)) {
        throw new Error(`VK generation failed for ${packageName} (noir-recursive)`)
      }
      result.vkNoir = existsSync(vkNoirFile) ? vkNoirFile : null
      result.vkNoirHash = existsSync(vkNoirHashFile) ? vkNoirHashFile : null
    } else {
      // Wrapper circuits: noir-recursive-no-zk (Default variant)
      if (!runWriteVk('noir-recursive-no-zk', vkRecursiveFile, vkRecursiveHashFile)) {
        throw new Error(`VK generation failed for ${packageName} (noir-recursive-no-zk)`)
      }
      result.vkRecursive = existsSync(vkRecursiveFile) ? vkRecursiveFile : null
      result.vkRecursiveHash = existsSync(vkRecursiveHashFile) ? vkRecursiveHashFile : null

      // noir-recursive VK: needed if wrapper/fold proofs are embedded in further recursive aggregation
      if (!runWriteVk('noir-recursive', vkNoirFile, vkNoirHashFile)) {
        throw new Error(`VK generation failed for ${packageName} (noir-recursive)`)
      }
      result.vkNoir = existsSync(vkNoirFile) ? vkNoirFile : null
      result.vkNoirHash = existsSync(vkNoirHashFile) ? vkNoirHashFile : null
    }

    return result
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
      const packageName = basename(c.artifacts.json ?? '', '.json')

      // evm/ variant checksums (only for circuits that have an evm VK)
      if (c.checksums.vk && c.artifacts.vk) {
        const evmPrefix = `${CIRCUIT_VARIANTS.EVM}/${c.group}/${c.name}`
        if (c.checksums.json && c.artifacts.json) {
          const f = `${evmPrefix}/${basename(c.artifacts.json)}`
          checksums[f] = c.checksums.json
          lines.push(`${c.checksums.json}  ${f}`)
        }
        const f = `${evmPrefix}/${basename(c.artifacts.vk)}`
        checksums[f] = c.checksums.vk
        lines.push(`${c.checksums.vk}  ${f}`)
        if (c.checksums.vkHash && c.artifacts.vkHash) {
          const fHash = `${evmPrefix}/${basename(c.artifacts.vkHash)}`
          checksums[fHash] = c.checksums.vkHash
          lines.push(`${c.checksums.vkHash}  ${fHash}`)
        }
      }

      // default/ variant checksums
      const defaultPrefix = `${CIRCUIT_VARIANTS.DEFAULT}/${c.group}/${c.name}`
      if (c.checksums.json && c.artifacts.json) {
        const f = `${defaultPrefix}/${basename(c.artifacts.json)}`
        checksums[f] = c.checksums.json
        lines.push(`${c.checksums.json}  ${f}`)
      }
      if (c.checksums.vkRecursive && c.artifacts.vkRecursive) {
        // In default/ variant, .vk_recursive is stored as .vk
        const f = `${defaultPrefix}/${packageName}.vk`
        checksums[f] = c.checksums.vkRecursive
        lines.push(`${c.checksums.vkRecursive}  ${f}`)
      }
      if (c.checksums.vkRecursiveHash && c.artifacts.vkRecursiveHash) {
        // In default/ variant, .vk_recursive_hash is stored as .vk_hash
        const f = `${defaultPrefix}/${packageName}.vk_hash`
        checksums[f] = c.checksums.vkRecursiveHash
        lines.push(`${c.checksums.vkRecursiveHash}  ${f}`)
      }
      // recursive/ variant checksums (noir-recursive VKs for inner proofs)
      if (c.checksums.vkNoir && c.artifacts.vkNoir) {
        const recursivePrefix = `${CIRCUIT_VARIANTS.RECURSIVE}/${c.group}/${c.name}`
        if (c.checksums.json && c.artifacts.json) {
          const f = `${recursivePrefix}/${basename(c.artifacts.json)}`
          checksums[f] = c.checksums.json
          lines.push(`${c.checksums.json}  ${f}`)
        }
        const fVk = `${recursivePrefix}/${packageName}.vk`
        checksums[fVk] = c.checksums.vkNoir
        lines.push(`${c.checksums.vkNoir}  ${fVk}`)
        if (c.checksums.vkNoirHash && c.artifacts.vkNoirHash) {
          const fHash = `${recursivePrefix}/${packageName}.vk_hash`
          checksums[fHash] = c.checksums.vkNoirHash
          lines.push(`${c.checksums.vkNoirHash}  ${fHash}`)
        }
      }
    }

    // share_computation wrapper aliases
    const wrapperPath = SHARE_COMP_WRAPPER.path.join('/')
    for (const variant of SHARE_COMP_WRAPPER.variants) {
      const shareCompPrefix = `${variant}/${wrapperPath}/share_computation`
      const basePrefix = `${variant}/${wrapperPath}`
      for (const alias of SHARE_COMP_WRAPPER.aliases) {
        for (const suffix of ['.json', '.vk', '.vk_hash']) {
          const srcKey = `${shareCompPrefix}/share_computation${suffix}`
          const srcHash = checksums[srcKey]
          if (srcHash) {
            const aliasKey = `${basePrefix}/${alias}/${alias}${suffix}`
            checksums[aliasKey] = srcHash
            lines.push(`${srcHash}  ${aliasKey}`)
          }
        }
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
      const packageName = basename(c.artifacts.json ?? '', '.json')

      // Copy to evm/ variant: .json + evm .vk + .vk_hash (only for circuits that have an evm VK)
      if (c.artifacts.vk) {
        const evmDir = join(outputDir, CIRCUIT_VARIANTS.EVM, c.group, c.name)
        mkdirSync(evmDir, { recursive: true })
        if (c.artifacts.json) copyFileSync(c.artifacts.json, join(evmDir, basename(c.artifacts.json)))
        copyFileSync(c.artifacts.vk, join(evmDir, basename(c.artifacts.vk)))
        if (c.artifacts.vkHash) copyFileSync(c.artifacts.vkHash, join(evmDir, basename(c.artifacts.vkHash)))
      }

      // Copy to default/ variant: .json + noir-recursive-no-zk .vk (wrapper/fold proofs)
      const defaultDir = join(outputDir, CIRCUIT_VARIANTS.DEFAULT, c.group, c.name)
      mkdirSync(defaultDir, { recursive: true })
      if (c.artifacts.json) copyFileSync(c.artifacts.json, join(defaultDir, basename(c.artifacts.json)))
      if (c.artifacts.vkRecursive) {
        copyFileSync(c.artifacts.vkRecursive, join(defaultDir, `${packageName}.vk`))
      }
      if (c.artifacts.vkRecursiveHash) {
        copyFileSync(c.artifacts.vkRecursiveHash, join(defaultDir, `${packageName}.vk_hash`))
      }

      // Copy to recursive/ variant: .json + noir-recursive .vk (inner/base proofs fed into wrapper)
      if (c.artifacts.vkNoir) {
        const recursiveDir = join(outputDir, CIRCUIT_VARIANTS.RECURSIVE, c.group, c.name)
        mkdirSync(recursiveDir, { recursive: true })
        if (c.artifacts.json) copyFileSync(c.artifacts.json, join(recursiveDir, basename(c.artifacts.json)))
        copyFileSync(c.artifacts.vkNoir, join(recursiveDir, `${packageName}.vk`))
        if (c.artifacts.vkNoirHash) {
          copyFileSync(c.artifacts.vkNoirHash, join(recursiveDir, `${packageName}.vk_hash`))
        }
      }
    }

    // Share_computation wrapper aliases (source includes circuit group per SHARE_COMP_WRAPPER.path)
    for (const variant of SHARE_COMP_WRAPPER.variants) {
      const shareCompSrc = join(outputDir, variant, ...SHARE_COMP_WRAPPER.path, 'share_computation')
      const hasJson = existsSync(join(shareCompSrc, 'share_computation.json'))
      const hasVk = existsSync(join(shareCompSrc, 'share_computation.vk'))
      const hasVkHash = existsSync(join(shareCompSrc, 'share_computation.vk_hash'))
      if (hasJson || hasVk || hasVkHash) {
        for (const alias of SHARE_COMP_WRAPPER.aliases) {
          const destDir = join(outputDir, variant, ...SHARE_COMP_WRAPPER.path, alias)
          mkdirSync(destDir, { recursive: true })
          if (hasJson) copyFileSync(join(shareCompSrc, 'share_computation.json'), join(destDir, `${alias}.json`))
          if (hasVk) copyFileSync(join(shareCompSrc, 'share_computation.vk'), join(destDir, `${alias}.vk`))
          if (hasVkHash) copyFileSync(join(shareCompSrc, 'share_computation.vk_hash'), join(destDir, `${alias}.vk_hash`))
        }
      }
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
