---
description: A node that is responsible for managing keyshares to form a decryption committee for enclave encrypted data
---

## `=this.file.name`

`=this.description`

## Ciphernode Map

```mermaid
flowchart TB
    subgraph s1["Ciphernode"]
        EVM["EvmSystem"]
        EB["EventBus"]
        NET["NetSystem"]
		R["E3RequestSystem"]
        KS["KeyshareSystem"]
        COM["ThreadpoolSystem"]
        P["PersistenceSystem"]
        AS["AggregationSystem"]
        SS["SortitionSystem"]
    end

	EB --- EVM
    EB --- NET
    R --- AS
    R --- KS
    EB --- R
    AS --- COM
    KS --- COM
    AS --- SS
    EB --- SS
    AS -.- P
    KS -.- P
    R -.- P

    EVM:::internal-link
    EB:::internal-link
    NET:::internal-link
    COM:::internal-link
    R:::internal-link
    AS:::internal-link
    KS:::internal-link
    SS:::internal-link
    P:::internal-link

    style EVM fill:#BBDEFB
    style EB fill:#FFCDD2
    style NET fill:#BBDEFB
    style R fill:#BBDEFB
	style KS fill:#BBDEFB
    style COM fill:#BBDEFB
    style AS fill:#BBDEFB
    style SS fill:#BBDEFB
	style P fill:#BBDEFB

    click EVM "http://github.com/gnosisguild/enclave/tree/main/crates/evm/EvmSystem.md"
    click EB "http://github.com/gnosisguild/enclave/tree/main/crates/events/docs/EventBus.md"
    click NET "http://github.com/gnosisguild/enclave/tree/main/crates/net/NetSystem.md"
    click COM "http://github.com/gnosisguild/enclave/tree/main/crates/threadpool/ThreadpoolSystem.md"
    click R "http://github.com/gnosisguild/enclave/tree/main/crates/request/E3RequestSystem.md"
    click AS "http://github.com/gnosisguild/enclave/tree/main/crates/aggregator/AggregationSystem.md"
    click KS "http://github.com/gnosisguild/enclave/tree/main/crates/keyshare/KeyshareSystem.md"
    click SS "http://github.com/gnosisguild/enclave/tree/main/crates/sortition/SortitionSystem.md"
    click P "http://github.com/gnosisguild/enclave/tree/main/crates/data/PersistenceSystem.md"
```
<details>
<summary>Links</summary>

[[AggregationSystem]]
[[E3RequestSystem]]
[[EventBus]]
[[EvmSystem]]
[[KeyshareSystem]]
[[NetSystem]]
[[PersistenceSystem]]
[[SortitionSystem]]
[[ThreadpoolSystem]]
</details>

## Design

A ciphernode is designed as an event driven actor model system. Some key considerations around this design decision are listed below.

- [[The Actor Model]]
- [[Event Driven Architecture]]
- [[PersistenceSystem|On Persistence]]
- [[Data Security]]

## Bootstrap

When you run `enclave start`, the CLI establishes an actor configuration based on your requirements. For a concrete implementation example, [see the start configuration](https://github.com/gnosisguild/enclave/blob/main/crates/entrypoint/src/start/start.rs)

This process instantiates several key components:

- An [[EventBus]] for system-wide message coordination
- [[EvmSystem]] actors that handle blockchain connectivity
- [[NetSystem]] components for peer-to-peer network communication
- Core E3 business logic components essential for proper system operation

The configuration ensures all necessary subsystems are properly initialized and can communicate effectively within the enclave architecture.

### Systems

```dataview
TABLE description as Description
WHERE type = "system"
```

### Resources

- [[Actors]]
- [[Events]]
