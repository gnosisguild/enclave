#!/usr/bin/env tsx

import { readFileSync, writeFileSync, existsSync } from 'fs'
import { join, resolve } from 'path'

interface PackageJson {
  name: string
  version: string
  [key: string]: any
}

class VersionBumper {
  private newVersion: string
  private rootDir: string

  constructor(newVersion: string) {
    this.newVersion = newVersion
    this.rootDir = resolve(__dirname, '..')
  }

  /**
   * Main entry point to bump all versions
   */
  async bumpAll(): Promise<void> {
    console.log(`🚀 Bumping all versions to ${this.newVersion}`)
    
    try {
      // Validate version format
      this.validateVersion(this.newVersion)
      
      // Bump Rust crates
      await this.bumpRustCrates()
      
      // Bump npm packages
      await this.bumpNpmPackages()
      
      console.log('✅ All versions bumped successfully!')
      console.log('\n📋 Summary:')
      console.log(`   Rust crates: ${this.newVersion}`)
      console.log(`   NPM packages: ${this.newVersion}`)
      console.log('\n💡 Next steps:')
      console.log('   1. Review the changes')
      console.log('   2. Commit the changes')
      console.log('   3. Run tests to ensure everything works')
      
    } catch (error) {
      console.error('❌ Error bumping versions:', error)
      process.exit(1)
    }
  }

  /**
   * Validate version format (semantic versioning)
   */
  private validateVersion(version: string): void {
    const semverRegex = /^\d+\.\d+\.\d+(-[a-zA-Z0-9.-]+)?(\+[a-zA-Z0-9.-]+)?$/
    if (!semverRegex.test(version)) {
      throw new Error(`Invalid version format: ${version}. Expected format: x.y.z[-prerelease][+build]`)
    }
  }

  /**
   * Bump versions in all Rust crates
   */
  private async bumpRustCrates(): Promise<void> {
    console.log('\n🦀 Bumping Rust crate versions...')
    
    // Update root Cargo.toml workspace version (this propagates to all crates)
    const rootCargoPath = join(this.rootDir, 'Cargo.toml')
    this.updateCargoToml(rootCargoPath, 'workspace.package.version')
    
    // Update workspace dependencies in root Cargo.toml
    this.updateWorkspaceDependencies(rootCargoPath)
    
    console.log('   ✓ All workspace crates (via workspace.version)')
  }

  /**
   * Bump versions in all npm packages
   */
  private async bumpNpmPackages(): Promise<void> {
    console.log('\n📦 Bumping NPM package versions...')
    
    // Main packages to bump (excluding examples and templates)
    const packagesToBump = [
      'packages/enclave-sdk',
      'packages/enclave-contracts', 
      'packages/enclave-config',
      'packages/enclave-react',
      'crates/wasm'
    ]
    
    for (const packagePath of packagesToBump) {
      const fullPath = join(this.rootDir, packagePath)
      const packageJsonPath = join(fullPath, 'package.json')
      
      if (existsSync(packageJsonPath)) {
        this.updatePackageJson(packageJsonPath)
        const packageName = this.getPackageName(packageJsonPath)
        console.log(`   ✓ ${packageName}`)
      }
    }
  }

  /**
   * Update Cargo.toml file
   */
  private updateCargoToml(filePath: string, versionKey: string): void {
    const content = readFileSync(filePath, 'utf-8')
    const lines = content.split('\n')
    
    let inTargetSection = false
    let updated = false
    
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i].trim()
      
      // Check if we're in the target section
      if (line === `[${versionKey.split('.').join('].')}]` || 
          (versionKey === 'workspace.package.version' && line === '[workspace.package]')) {
        inTargetSection = true
        continue
      }
      
      // If we're in the target section and find a version line
      if (inTargetSection && line.startsWith('version = ')) {
        lines[i] = `version = "${this.newVersion}"`
        updated = true
        break
      }
      
      // Reset section flag when we hit a new section
      if (line.startsWith('[') && inTargetSection) {
        break
      }
    }
    
    if (updated) {
      writeFileSync(filePath, lines.join('\n'))
    } else {
      console.warn(`⚠️  Could not find version in ${filePath}`)
    }
  }

  /**
   * Update workspace dependencies in root Cargo.toml
   */
  private updateWorkspaceDependencies(filePath: string): void {
    const content = readFileSync(filePath, 'utf-8')
    const lines = content.split('\n')
    
    let inWorkspaceDeps = false
    let updated = false
    
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i].trim()
      
      if (line === '[workspace.dependencies]') {
        inWorkspaceDeps = true
        continue
      }
      
      if (inWorkspaceDeps && line.startsWith('version = ')) {
        lines[i] = `version = "${this.newVersion}"`
        updated = true
      }
      
      // Reset when we hit a new section
      if (line.startsWith('[') && inWorkspaceDeps && line !== '[workspace.dependencies]') {
        break
      }
    }
    
    if (updated) {
      writeFileSync(filePath, lines.join('\n'))
    }
  }

  /**
   * Update package.json file
   */
  private updatePackageJson(filePath: string): void {
    const content = readFileSync(filePath, 'utf-8')
    const packageJson: PackageJson = JSON.parse(content)
    
    packageJson.version = this.newVersion
    
    // Write back with proper formatting
    writeFileSync(filePath, JSON.stringify(packageJson, null, 2) + '\n')
  }

  /**
   * Get package name from package.json
   */
  private getPackageName(filePath: string): string {
    const content = readFileSync(filePath, 'utf-8')
    const packageJson: PackageJson = JSON.parse(content)
    return packageJson.name || 'unknown'
  }

}

// CLI interface
async function main() {
  const args = process.argv.slice(2)
  
  if (args.length === 0) {
    console.log('Usage: tsx scripts/bump-versions.ts <version>')
    console.log('Example: tsx scripts/bump-versions.ts 1.0.0')
    console.log('Example: tsx scripts/bump-versions.ts 1.0.0-beta.1')
    process.exit(1)
  }
  
  const version = args[0]
  const bumper = new VersionBumper(version)
  await bumper.bumpAll()
}

// Run if called directly
if (require.main === module) {
  main().catch(console.error)
}

export { VersionBumper }
