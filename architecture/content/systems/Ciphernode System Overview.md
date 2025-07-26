---
description: Demonstrate an overview of the structure of the ciphernode component system
---
## `=this.file.name`

`=this.description`


```mermaid
flowchart TB
    subgraph s1["Ciphernode"]
        EVM["EvmSystem"]
        EB["EventBus"]
        NET["NetSystem"]
		R["E3RequestSystem"]
        KS["KeyshareSystem"]
        COM["ComputeSystem"]
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
```



### Systems

```dataview
TABLE description as Description
WHERE type = "system"
```


## Bootstrap

When you run `enclave start`, the CLI establishes an actor configuration based on your  requirements. For a concrete implementation example, [see the start configuration](https://github.com/gnosisguild/enclave/blob/main/crates/entrypoint/src/start/start.rs) 

This process instantiates several key components:

- An [[EventBus]] for system-wide message coordination
- [[EvmSystem]] actors that handle blockchain connectivity
- [[NetSystem]] components for peer-to-peer network communication
- Core E3 business logic components essential for proper system operation

The configuration ensures all necessary subsystems are properly initialized and can communicate effectively within the enclave architecture.