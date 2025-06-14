# enclaveup

A standalone installer for the Enclave CLI tool.

## Installation

### Quick Install

Use the provided install script to download and install `enclaveup`:

```bash
curl -fsSL https://raw.githubusercontent.com/gnosisguild/enclave/main/install | bash
```

Or with wget:

```bash
wget -qO- https://raw.githubusercontent.com/gnosisguild/enclave/main/install | bash
```

### Manual Installation

1. Download the appropriate binary for your platform from the [releases page](https://github.com/gnosisguild/enclave/releases)
2. Extract the binary and place it in your PATH (e.g., `~/.local/bin` or `/usr/local/bin`)
3. Make sure the binary is executable: `chmod +x enclaveup`

## Usage

### Install the Enclave CLI

```bash
# Install to ~/.local/bin (default)
enclaveup install

# Install to /usr/local/bin (requires sudo)
enclaveup install --system
```

### Update the Enclave CLI

```bash
# Update from ~/.local/bin
enclaveup update

# Update from /usr/local/bin
enclaveup update --system
```

### Uninstall the Enclave CLI

```bash
# Remove from ~/.local/bin
enclaveup uninstall

# Remove from /usr/local/bin
enclaveup uninstall --system
```

### Get Help

```bash
enclaveup --help
enclaveup install --help
```

## Building from Source

To build `enclaveup` from source:

```bash
cd enclaveup
cargo build --release
```

The binary will be available at `target/release/enclaveup`.

## Platform Support

| Platform | Architecture | Status |
|----------|-------------|---------|
| Linux | x86_64 | ✅ Native binary |
| macOS | Apple Silicon (M1/M2/M3) | ✅ Native binary |
| macOS | Intel | ✅ Via Rosetta 2* |

\* Intel Macs automatically run Apple Silicon binaries through Rosetta 2 translation

## Binary Naming Convention

The installer expects GitHub releases to contain assets with this naming pattern:

**For Enclave CLI:**
- `enclave-linux-x86_64.tar.gz`
- `enclave-macos-aarch64.tar.gz`

**For enclaveup itself:**
- `enclaveup-linux-x86_64.tar.gz`
- `enclaveup-macos-aarch64.tar.gz`

Each tarball contains the binary at the root level.

## Dependencies

- `curl` or `wget` (for the install script)
- `tar` (for extracting archives)
- Internet connection (for downloading releases)