# Scripts

This directory contains utility scripts for the Enclave project.

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
