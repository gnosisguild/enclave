# SAFE (Sponge API for Field Elements) Noir Library

This package contains a complete implementation of the SAFE API in Noir as defined in the specification [SAFE (Sponge API for Field Elements) - A Toolbox for ZK Hash Applications](https://hackmd.io/bHgsH6mMStCVibM_wYvb2w#22-Sponge-state). SAFE provides a unified interface for cryptographic sponge functions that can be instantiated with various permutations to create hash functions, MACs, authenticated encryption schemes, and other cryptographic primitives for ZK proof systems.

- START, ABSORB, SQUEEZE, FINISH operations following spec 2.4
- Domain separation, tag computation, IO pattern validation
- Field-friendly permutation for ZK systems
- All operations follow SAFE spec 2.4 exactly
- Variable-length inputs, automatic length detection from IO patterns
- Automatic validation of operation sequences against expected patterns
- Cross-protocol security through configurable domain separators

## Installation

In your _Nargo.toml_ file, add this library as a dependency:

```toml
[dependencies]
safe = { tag = "v0.1.5", git = "https://github.com/gnosisguild/enclave", directory = "packages/circuits/crates/libs/safe"}
```

nb. the `tag` corresponds to the latest tag release of Enclave (`v0.1.5`). From `v0.1.6` you should remove `packages/` from `directory` field (ie., `circuits/crates/...`).

## API Reference

### SafeSponge<L>

The main sponge structure that implements the SAFE API.

#### Methods

- `start(io_pattern: [u32; L], domain_separator: [u8; 64]) -> SafeSponge<L>`: Initializes a new SAFE sponge instance with the given IO pattern and domain separator

- `absorb(&mut self, input: [Field])`: Absorbs field elements into the sponge state, automatically validating against the IO pattern

- `squeeze(&mut self) -> Vec<Field>`: Extracts field elements from the sponge state according to the IO pattern

- `finish(&mut self)`: Finalizes the sponge instance, verifying all operations and clearing internal state

### IO Pattern Encoding

IO patterns are encoded as 32-bit words where:

- MSB = 1 for ABSORB operations
- MSB = 0 for SQUEEZE operations
- Lower 31 bits specify the number of field elements

Examples:

- `0x80000003` = ABSORB(3)
- `0x00000001` = SQUEEZE(1)
- `0x80000000` = ABSORB(0)

### Domain Separation

Each sponge instance requires a 64-byte domain separator for cross-protocol security. Different domain separators ensure that distinct applications behave like distinct functions.

## Compatibility

This has been developed and tested with

```bash
nargo --version
nargo version = 1.0.0-beta.15
noirc version = 1.0.0-beta.15+83245db91dcf63420ef4bcbbd85b98f397fee663
(git version hash: 83245db91dcf63420ef4bcbbd85b98f397fee663, is dirty: false)
```

```bash
bb --version
3.0.0-nightly.20251104
```
