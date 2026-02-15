# Scripts

This directory contains utility scripts for the Enclave project.

## Version Bumper

`bump-versions.ts` - Bumps the versions of all packages and crates in the project.

### Usage

```bash
# Full release (bump, commit, tag, and push)
pnpm bump:versions 1.0.0

# Pre-release version
pnpm bump:versions 1.0.0-beta.1

# Local only (don't push to remote)
pnpm bump:versions --no-push 1.0.0

# Manual git operations (just bump versions)
pnpm bump:versions --skip-git 1.0.0

# Test run (see what would happen)
pnpm bump:versions --dry-run 1.0.0
```

### What it does

**By default, the script performs a complete release:**

1. **Validates** your working directory is clean (no uncommitted changes)
2. **Updates versions** across the entire monorepo:
   - Rust workspace version in root `Cargo.toml`
   - All npm packages in `packages/` and `crates/wasm`
   - Root `package.json`
3. **Updates lock files**:
   - `Cargo.lock` for Rust dependencies
   - `pnpm-lock.yaml` for npm dependencies
4. **Generates changelog** from conventional commits (uses `CHANGELOG.md`)
5. **Commits** all changes with message: `chore(release): bump version to X.Y.Z`
6. **Creates** annotated git tag: `vX.Y.Z`
7. **Pushes** commits and tag to GitHub
8. **Triggers** the automated release workflow

### Examples

```bash
# One-command release (recommended)
pnpm bump:versions 1.2.3
# This bumps everything, commits, tags, and pushes - triggering the full release!

# Pre-release for testing
pnpm bump:versions 1.2.3-beta.1
# Automatically detected as pre-release, published to npm with 'next' tag

# Prepare release locally first
pnpm bump:versions --no-push 1.2.3
# Does everything except push - review first, then: git push && git push --tags

# Just bump versions (old behavior)
pnpm bump:versions --skip-git 1.2.3
# Only updates versions, you handle git operations manually
```

### Options

- `--skip-git` - Skip all git operations (add, commit, tag, push)
- `--no-push` - Perform git operations locally but don't push
- `--dry-run` - Preview what would happen without making any changes
- `--help` - Show help message

### Prerequisites

- Clean working directory (no uncommitted changes)
- Conventional commits for changelog generation
- Valid semver version format

### After Running

Once you run `pnpm bump:versions X.Y.Z` and the tag is pushed, GitHub Actions automatically:

- Builds binaries for all platforms (Linux, macOS)
- Publishes to npm (with `latest` or `next` tag)
- Publishes to crates.io
- Creates GitHub release with changelog and binaries

## License Header Checker

`check-license-headers.sh` - Checks and fixes SPDX license headers in source files.

### Usage

```bash
# Check all files for license headers
./scripts/check-license-headers.sh

# Automatically fix missing headers
./scripts/check-license-headers.sh --fix

# Check only (for CI/CD, exits with code 1 if issues found)
./scripts/check-license-headers.sh --check-only
```

### What it does

- Scans all `.rs`, `.sol`, and `.ts` files in the repository
- Excludes certain files with different licensing (e.g., `ImageID.sol` from RISC Zero with Apache
  license)
- Checks for the required SPDX license header:
  ```
  // SPDX-License-Identifier: LGPL-3.0-only
  //
  // This file is provided WITHOUT ANY WARRANTY;
  // without even the implied warranty of MERCHANTABILITY
  // or FITNESS FOR A PARTICULAR PURPOSE.
  ```
- In `--fix` mode, automatically adds the header to files that are missing it
- Skips files that already have an SPDX header (these need manual review)
- Excludes common build/dependency directories (`node_modules`, `target`, etc.)

### CI/CD Integration

This script is automatically run in GitHub Actions:

- On pull requests: checks headers and comments if any are missing
- On pushes to main/develop: automatically fixes missing headers and commits changes

## Clean Script

`clean.ts` - Removes build artifacts and temporary files from the repository using predefined safe
patterns while providing options to skip specific parts of the codebase.

### Usage

```bash
# Clean build artifacts
pnpm clean

# Dry run to see what would be cleaned
pnpm clean --dry-run

# Clean everything except crates and contracts
pnpm clean --skip-crates --skip-contracts

# Interactive cleaning
pnpm clean --interactive

# Show help message
pnpm clean --help
```

### What it does

- **Uses predefined patterns** to identify safe-to-clean build artifacts and temporary files
- **Safely removes** only files matching known safe patterns (node_modules, dist, target, etc.)
- **Provides granular control** over what gets cleaned via skip options
- **Shows detailed statistics** about what was removed and space freed
- **Prevents accidental deletion** of important files by using a whitelist approach

## Circuit Builder

`build-circuits.ts` - Compiles Noir circuits, generates verification keys, and prepares release
artifacts.

### Usage

```bash
# Build all circuits
pnpm build:circuits

# Build only specific group (dkg or threshold)
pnpm build:circuits --group dkg

# Skip verification key generation (faster)
pnpm build:circuits --skip-vk

# Dry run to see what would be built
pnpm build:circuits --dry-run

# Get source hash for change detection
pnpm build:circuits hash
```

### What it does

1. **Discovers circuits** in `circuits/bin/dkg/` and `circuits/bin/threshold/`
2. **Compiles** each circuit using `nargo compile`
3. **Generates verification keys** using `bb write_vk`
4. **Sanitizes paths** in compiled JSON (removes local filesystem paths for opsec)
5. **Generates checksums** (`SHA256SUMS` and `checksums.json`)
6. **Copies artifacts** to `dist/circuits/`

### Options

- `--group <groups>` - Circuit groups (comma-separated: dkg,threshold)
- `--circuit <name>` - Build specific circuit(s)
- `--skip-vk` - Skip verification key generation
- `--skip-checksums` - Skip checksum generation
- `-o, --output <dir>` - Output directory (default: dist/circuits)
- `--dry-run` - Show what would be built
- `--no-clean` - Don't clean output directory

### Prerequisites

- `nargo` - Noir compiler ([install](https://noir-lang.org/docs/getting_started/installation/))
- `bb` - Barretenberg prover (for verification keys)

## Circuit Artifacts

`circuit-artifacts.ts` - Push/pull pre-built circuit artifacts via git branch.

### Usage

```bash
# Build circuits locally, then push to git branch
pnpm build:circuits
pnpm store:circuits push

# Pull circuits from git branch (used by CI)
pnpm store:circuits pull
```

### What it does

- **Push**: Copies `dist/circuits/` to the `circuit-artifacts` orphan branch and pushes to origin
- **Pull**: Fetches the `circuit-artifacts` branch and extracts to `dist/circuits/`

### Workflow

Circuits are built locally and stored in a git branch:

1. **Local**: Build circuits and push to branch

```bash
   pnpm build:circuits
   pnpm tsx scripts/circuit-artifacts.ts push
```

2. **CI**: Pulls from branch during release, attaches to GitHub release

3. **After release**: Circuits live permanently in release assets

## Verifier Generator

`generate-verifiers.ts` - Generates Solidity verifier contracts from compiled Noir circuits.

### Usage

```bash
# Generate verifiers for all circuits
pnpm generate:verifiers

# Generate only for specific group
pnpm generate:verifiers --group dkg
pnpm generate:verifiers --group threshold

# Generate for specific circuit(s)
pnpm generate:verifiers --circuit pk
pnpm generate:verifiers --circuit pk --circuit fold

# Clean existing verifiers before generating
pnpm generate:verifiers --clean

# Preview what would be generated
pnpm generate:verifiers --dry-run

# Skip auto-compilation (requires pre-built circuits)
pnpm generate:verifiers --no-compile

# Specify oracle hash scheme for VK generation
pnpm generate:verifiers --oracle-hash keccak
```

### What it does

Automates the full pipeline from Noir circuits to on-chain Solidity verifiers:

1. **Discovers circuits** in `circuits/bin/{dkg,threshold,recursive_aggregation}/`
2. **Compiles circuits** with `nargo compile` (if not already compiled)
3. **Generates verification keys** using `bb write_vk --oracle_hash keccak`
4. **Generates Solidity verifiers** using `bb write_solidity_verifier`
5. **Post-processes** the generated Solidity:
   - Renames contract from `HonkVerifier` to descriptive name (e.g., `DkgPkVerifier`,
     `ThresholdPkGenerationVerifier`)
   - Replaces Apache-2.0 license header with LGPL-3.0-only
6. **Outputs** to `packages/enclave-contracts/contracts/verifier/`

### Options

- `--group <groups>` - Circuit groups (comma-separated: dkg,threshold,recursive_aggregation)
- `--circuit <name>` - Generate verifier for specific circuit(s) (repeatable)
- `--clean` - Remove existing verifier directory before generating
- `--no-compile` - Don't compile circuits automatically (fail if not already compiled)
- `--oracle-hash <hash>` - Oracle hash scheme for VK generation (default: keccak)
- `--dry-run` - Show what would be generated without doing anything
- `-h, --help` - Show help message

### Prerequisites

- `nargo` - Noir compiler ([install](https://noir-lang.org/docs/getting_started/installation/))
- `bb` - Barretenberg CLI for proof system operations

### Output Example

```
🔮 Generating Solidity verifiers from Noir circuits...

   Found 13 circuit(s)

   ✓ dkg/pk → DkgPkVerifier.sol
   ✓ dkg/sk_share_computation → DkgSkShareComputationVerifier.sol
   ✓ threshold/pk_generation → ThresholdPkGenerationVerifier.sol
   ...

✅ Generated 13 Solidity verifier(s) in:
   packages/enclave-contracts/contracts/verifier/
```

### Integration

Generated verifiers are automatically:

- Compiled with aggressive size optimization (`runs: 1` in Hardhat config)
- Deployed via `pnpm deploy` (integrated into main deployment flow)
- Saved to `deployed_contracts.json`
- Verified on block explorers via `pnpm verify:contracts`

### Notes

- Verifier contracts are large (~24KB) due to pairing cryptography
- Library linking (e.g., `ZKTranscriptLib`) is handled automatically during deployment
- Generated files are excluded from linting (`.solhintignore`)
