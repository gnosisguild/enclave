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
- Excludes certain files with different licensing (e.g., `ImageID.sol` from RISC Zero with Apache license)
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

`clean.ts` - Removes build artifacts and temporary files from the repository using predefined safe patterns while providing options to skip specific parts of the codebase.

### Usage

```bash
# Clean build artifacts (interactive mode)
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
