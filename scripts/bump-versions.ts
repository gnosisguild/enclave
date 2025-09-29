#!/usr/bin/env tsx

import { readFileSync, writeFileSync, existsSync } from 'fs'
import { join, resolve } from 'path'

interface PackageJson {
  name: string
  version: string
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
  bumpAll(): void {
    console.log(`🚀 Bumping all versions to ${this.newVersion}`)
    
    try {
      // Validate version format
      this.validateVersion(this.newVersion)
      
      // Bump Rust crates
      this.bumpRustCrates()
      
      // Bump npm packages
      this.bumpNpmPackages()
      
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
  private bumpRustCrates(): void {
    console.log('\n🦀 Bumping Rust crate versions...')
    
    // Update root Cargo.toml workspace version (this propagates to all crates)
    const rootCargoPath = join(this.rootDir, 'Cargo.toml')
    this.updateCargoToml(rootCargoPath)
    
    // Update workspace dependencies in root Cargo.toml
    this.updateWorkspaceDependencies(rootCargoPath)
    
    console.log('   ✓ All workspace crates (via workspace.version)')
  }

  /**
   * Bump versions in all npm packages
   */
  private bumpNpmPackages(): void {
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
   * Update Cargo.toml file (workspace version and dependencies)
   */
  private updateCargoToml(filePath: string): void {
    const content = readFileSync(filePath, 'utf-8')
    const lines = content.split('\n')
    
    let updated = false
    
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i].trim()
      
      // Update workspace package version
      if (line === '[workspace.package]') {
        // Look for version in the next few lines
        for (let j = i + 1; j < Math.min(i + 10, lines.length); j++) {
          if (lines[j].trim().startsWith('version = ')) {
            lines[j] = `version = "${this.newVersion}"`
            updated = true
            break
          }
        }
      }
      
      // Update workspace dependencies
      if (line === '[workspace.dependencies]') {
        // Look for dependency lines with inline versions
        for (let j = i + 1; j < lines.length; j++) {
          const depLine = lines[j].trim()
          
          // Skip empty lines and new sections
          if (depLine === '' || depLine.startsWith('[')) {
            break
          }
          
          // Update lines that have version = "..." in them
          if (depLine.includes('version = ')) {
            // Replace the version part while preserving the rest
            const updatedLine = depLine.replace(/version = "[^"]*"/, `version = "${this.newVersion}"`)
            lines[j] = updatedLine
            updated = true
          }
        }
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
function main() {
  const args = process.argv.slice(2)
  
  if (args.length === 0) {
    console.log('Usage: tsx scripts/bump-versions.ts <version>')
    console.log('Example: tsx scripts/bump-versions.ts 1.0.0')
    console.log('Example: tsx scripts/bump-versions.ts 1.0.0-beta.1')
    process.exit(1)
  }
  
  const version = args[0]
  const bumper = new VersionBumper(version)
  bumper.bumpAll()
}

// Run if called directly
if (require.main === module) {
  main()
}

export { VersionBumper }
