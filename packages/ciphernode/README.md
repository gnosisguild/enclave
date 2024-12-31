# Prerequisites

Tested prerequisite versions:

```
$ rustc --version
rustc 1.81.0 (eeb90cda1 2024-09-04)
```

We need `solc` available for compiling contract fixtures that are used under test

```
$ solc --version
solc, the solidity compiler commandline interface
Version: 0.8.21+commit.d9974bed.Linux.g++
```

We need foundry's `anvil` to test our evm facing rust code:

```
anvil --version
anvil 0.2.0 (9501589 2024-10-30T00:22:24.181391729Z)
```

Note some older versions of `anvil` are not compatible and can cause errors - by older I mean before this particuar dated patch `(9501589 2024-10-30T00:22:24.181391729Z)` - note they are not using semver.

If you get an error that looks similar to the following you likely need to upgrade your anvil version:

```
ERROR! event=EnclaveError(EnclaveError { err_type: Evm, message: "deserialization error: duplicate field `status` at line 1 column 34542" })
```

# E3 Requested

```mermaid
sequenceDiagram
    autonumber
    participant EVM as EVM
    participant CS as CiphernodeSelector
    participant E3 as E3Router
    participant KS as Keyshare
    participant PKA as PublicKeyAggregator
    participant S as Sortition

    EVM--)CS: E3Requested
    CS->>+S: has node?
    S--)-CS: yes
    CS--)E3: CiphernodeSelected
    E3->>PKA: Create new PublicKeyAggreator for this e3_id
    E3->>KS: Create new Keyshare for this e3_id
    loop
        KS--)PKA: KeyshareCreated
        PKA->>+S: has node?
        S--)-PKA: yes
    end
    PKA--)EVM: PublicKeyAggregated
    PKA--)PKA: Stop
```

# Ciphertext output published

```mermaid
sequenceDiagram
    autonumber
    participant EVM as EVM
    participant E3 as E3Router
    participant KS as Keyshare
    participant PTA as PlaintextAggregator
    participant S as Sortition

    EVM--)E3: CiphertextOutputPublished
    E3->>PTA: Create new PlaintextAggreator for this e3_id
    loop
        KS--)PTA: DecryptionShareCreated
        PTA->>+S: has node?
        S--)-PTA: yes
    end
    PTA--)EVM: PlaintextAggregated
    PTA--)+KS: PlaintextAggregated
    PTA--)PTA: Stop
    KS--)-KS: Stop
```

# Debugging

You can debug using the `-vvv` cli arguments to alter what output is produced by the node


```
# Show INFO logging
enclave start
```

```
# Show DEBUG logging
enclave start -v
```

```
# Show TRACE logging
enclave start -vv
```

if you want to remove all output aside from errors you can use the `--quiet` argument.


```
enclave --quiet
```
