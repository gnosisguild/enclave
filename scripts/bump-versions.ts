#!/usr/bin/env tsx
// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { readFileSync, writeFileSync, existsSync } from 'fs'
import { join, resolve } from 'path'
import { execSync } from 'child_process'

interface PackageJson {
  name: string
  version: string
}

class VersionBumper {
  private newVersion: string
  private oldVersion: string | null = null
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
      
      // Get current version from root package.json or Cargo.toml
      this.oldVersion = this.getCurrentVersion()
      console.log(`📌 Current version: ${this.oldVersion || 'unknown'}`)
      
      // Bump Rust crates
      this.bumpRustCrates()
      
      // Bump npm packages
      this.bumpNpmPackages()
      
      // Update lock files
      this.updateLockFiles()
      
      // Generate changelog
      await this.generateChangelog()
      
      console.log('\n✅ All versions bumped successfully!')
      console.log('\n📋 Summary:')
      console.log(`   Previous version: ${this.oldVersion || 'unknown'}`)
      console.log(`   New version: ${this.newVersion}`)
      console.log(`   Rust crates: ✓`)
      console.log(`   NPM packages: ✓`)
      console.log(`   Lock files: ✓`)
      console.log(`   Changelog: ✓`)
      
      console.log('\n💡 Next steps:')
      console.log('   1. Review the changes and CHANGELOG.md')
      console.log('   2. Commit: git add . && git commit -m "chore(release): bump version to ' + this.newVersion + '"')
      console.log('   3. Tag: git tag v' + this.newVersion)
      console.log('   4. Push: git push && git push --tags')
      
    } catch (error) {
      console.error('❌ Error bumping versions:', error)
      process.exit(1)
    }
  }

  /**
   * Get current version from the monorepo
   */
  private getCurrentVersion(): string | null {
    // Try to get from root package.json first
    const rootPackagePath = join(this.rootDir, 'package.json')
    if (existsSync(rootPackagePath)) {
      const content = readFileSync(rootPackagePath, 'utf-8')
      const packageJson = JSON.parse(content)
      if (packageJson.version) {
        return packageJson.version
      }
    }
    
    // Try to get from root Cargo.toml workspace version
    const rootCargoPath = join(this.rootDir, 'Cargo.toml')
    if (existsSync(rootCargoPath)) {
      const content = readFileSync(rootCargoPath, 'utf-8')
      const versionMatch = content.match(/\[workspace\.package\][\s\S]*?version = "([^"]+)"/)
      if (versionMatch) {
        return versionMatch[1]
      }
    }
    
    return null
  }

  /**
   * Generate changelog using conventional commits
   */
  private async generateChangelog(): Promise<void> {
    console.log('\n📝 Generating changelog...')
    
    try {
        execSync('pnpm conventional-changelog --help', { 
          stdio: 'ignore',
          cwd: this.rootDir
        })
      
      const changelogPath = join(this.rootDir, 'CHANGELOG.md')
      
      if (!existsSync(changelogPath)) {
        // First time - generate entire changelog
        console.log('   Generating full changelog from git history...')
        execSync('npx conventional-changelog -p angular -i CHANGELOG.md -s -r 0', {
          cwd: this.rootDir,
          stdio: 'inherit'
        })
      } else {
        // Update existing changelog with changes since last release
        console.log('   Updating changelog with new changes...')
        execSync('npx conventional-changelog -p angular -i CHANGELOG.md -s', {
          cwd: this.rootDir,
          stdio: 'inherit'
        })
      }
      
      console.log('   ✓ Changelog generated successfully')
      
    } catch (error) {
      console.warn('   ⚠️  Could not generate changelog:', error)
      console.log('   Continuing without changelog...')
    }
  }

  /**
   * Update lock files after version bump
   */
  private updateLockFiles(): void {
    console.log('\n🔒 Updating lock files...')
    
    // Update Cargo.lock
    try {
      execSync('cargo update --workspace', {
        cwd: this.rootDir,
        stdio: 'pipe'
      })
      console.log('   ✓ Cargo.lock updated')
    } catch (error) {
      console.warn('   ⚠️  Could not update Cargo.lock')
    }
    
    // Detect and update the appropriate Node.js lock file
    const pnpmLockPath = join(this.rootDir, 'pnpm-lock.yaml')
    
    if (existsSync(pnpmLockPath)) {
      try {
        execSync('pnpm install --lockfile-only', {
          cwd: this.rootDir,
          stdio: 'pipe'
        })
        console.log('   ✓ pnpm-lock.yaml updated')
      } catch (error) {
        console.warn('   ⚠️  Could not update pnpm-lock.yaml')
      }
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
    
    console.log('   ✓ All workspace crates updated via workspace.version')
  }

  /**
   * Bump versions in all npm packages
   */
  private bumpNpmPackages(): void {
    console.log('\n📦 Bumping NPM package versions...')
    
    // Update root package.json if it exists
    const rootPackagePath = join(this.rootDir, 'package.json')
    if (existsSync(rootPackagePath)) {
      this.updatePackageJson(rootPackagePath)
      console.log('   ✓ Root package.json')
    }
    
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
  
  if (args.length === 0 || args[0] === '--help' || args[0] === '-h') {
    console.log(`
Usage: tsx scripts/bump-versions.ts <version>

Version Bump Script for Enclave Monorepo
- Bumps all Rust crates and npm packages to the same version
- Generates changelog from conventional commits
- Updates lock files

Examples:
  tsx scripts/bump-versions.ts 1.0.0
  tsx scripts/bump-versions.ts 1.0.0-beta.1
  tsx scripts/bump-versions.ts 2.3.4

The script will:
  1. Update versions in all Rust crates (via workspace.version)
  2. Update versions in all npm packages
  3. Update lock files (Cargo.lock, package-lock.json, etc.)
  4. Generate/update CHANGELOG.md from git history

Commit Convention for Changelog:
  Use conventional commits for automatic changelog generation:
  - feat: New feature
  - fix: Bug fix
  - docs: Documentation changes
  - chore: Maintenance tasks
  - test: Test changes
  - refactor: Code refactoring
  - perf: Performance improvements
  - BREAKING CHANGE: Breaking changes
`)
    process.exit(args.length === 0 ? 1 : 0)
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
