import fs from 'fs'
import path from 'path'
import { fileURLToPath } from 'url'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)

const WORKSPACE_ROOT = path.join(__dirname, '..', '..')

interface Package {
  name: string
  path: string
}

interface PackageJson {
  name: string
  version: string
  dependencies?: Record<string, string>
  devDependencies?: Record<string, string>
  peerDependencies?: Record<string, string>
  [key: string]: any
}

const PACKAGES: Package[] = [
  { name: '@enclave-e3/wasm', path: 'crates/wasm' },
  { name: '@enclave-e3/config', path: 'packages/enclave-config' },
  { name: '@enclave-e3/contracts', path: 'packages/enclave-contracts' },
  { name: '@enclave-e3/sdk', path: 'packages/enclave-sdk' },
  { name: '@enclave-e3/react', path: 'packages/enclave-react' },
]

console.log('🔧 Preparing packages for npm publishing...\n')

// Build a map of package names to versions
const packageVersions = new Map<string, string>()
PACKAGES.forEach(({ name, path: pkgPath }) => {
  const pkgJsonPath = path.join(WORKSPACE_ROOT, pkgPath, 'package.json')
  const pkgJson: PackageJson = JSON.parse(fs.readFileSync(pkgJsonPath, 'utf8'))
  packageVersions.set(name, pkgJson.version)
  console.log(`📦 Found ${name}@${pkgJson.version}`)
})

console.log('\n🔄 Replacing workspace:* dependencies...\n')

// Replace workspace:* with actual versions
PACKAGES.forEach(({ name, path: pkgPath }) => {
  const pkgJsonPath = path.join(WORKSPACE_ROOT, pkgPath, 'package.json')
  const pkg: PackageJson = JSON.parse(fs.readFileSync(pkgJsonPath, 'utf8'))

  let hasChanges = false

  const replaceWorkspaceDeps = (deps: Record<string, string> | undefined, depType: string): void => {
    if (!deps) return

    for (const [depName, version] of Object.entries(deps)) {
      if (version.startsWith('workspace:')) {
        const actualVersion = packageVersions.get(depName)
        if (actualVersion) {
          deps[depName] = `^${actualVersion}`
          console.log(`  ✓ ${name} ${depType}: ${depName}: ${version} → ^${actualVersion}`)
          hasChanges = true
        } else {
          console.warn(`  ⚠️  Warning: Could not find version for ${depName}`)
        }
      }
    }
  }

  replaceWorkspaceDeps(pkg.dependencies, 'dependencies')
  replaceWorkspaceDeps(pkg.devDependencies, 'devDependencies')
  replaceWorkspaceDeps(pkg.peerDependencies, 'peerDependencies')

  if (hasChanges) {
    fs.writeFileSync(pkgJsonPath, JSON.stringify(pkg, null, 2) + '\n')
    console.log(`  ✅ Updated ${pkgPath}/package.json\n`)
  } else {
    console.log(`  ⏭️  No workspace dependencies in ${name}\n`)
  }
})

console.log('✨ All packages prepared for npm publishing!\n')
