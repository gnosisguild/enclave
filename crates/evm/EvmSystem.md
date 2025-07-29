---
type: system
description: Handles conversion between live on-chain events and `EnclaveEvent` application events
---
## `=this.file.name`

`=this.description`


```mermaid
flowchart TB
    subgraph evm["Listening on-chain"]
		EB["EventBus"]
        ESR["EnclaveSolReader"]
        ER["EventReader"]
        EE["EnclaveEvent"]
        CRR["CiphernodeRegistryReader"]
    end
	CRR --> ER
	ESR --> ER
	ER --> EE
	EE --> EB
    
    ESR:::internal-link
    ER:::internal-link
    CRR:::internal-link
    EB:::internal-link
    EE:::internal-link
    
	style ESR fill:#FFCDD2
	style ER fill:#FFCDD2
	style CRR fill:#FFCDD2
	style EB fill:#FFCDD2

    click ESR "http://github.com/gnosisguild/enclave/tree/main/crates/evm/docs/EnclaveSolReader.md"
    click ER "http://github.com/gnosisguild/enclave/tree/main/crates/evm/docs/EventReader.md"
    click CRR "http://github.com/gnosisguild/enclave/tree/main/crates/evm/docs/CiphernodeRegistryReader.md"
    click EB "http://github.com/gnosisguild/enclave/tree/main/crates/events/docs/EventBus.md"
    click EE "http://github.com/gnosisguild/enclave/tree/main/crates/events/docs/EnclaveEvent.md"
```
<details>
<summary><i>Links</i></summary>

[[CiphernodeRegistryReader]]
[[EnclaveEvent]]
[[EnclaveSolReader]]
[[EventBus]]
[[EventReader]]
</details>
<details>
<summary><i>Links</i></summary>

[[CiphernodeRegistryReader]]
[[EnclaveSolReader]]
[[EventBus]]
[[EventReader]]
</details>

```mermaid
flowchart TB
    subgraph evm["EVM Writers"]
		EB["EventBus"]
		EE["EnclaveEvent"]
		ESW["EnclaveSolWriter"]
        RFW["RegistryFilterSolWriter"]
    end

	EB --> EE
	EE --> ESW
	EE --> RFW
	
    ESW:::internal-link
    EB:::internal-link
	RFW:::internal-link
	EE:::internal-link
	
	style ESW fill:#FFCDD2
	style EB fill:#FFCDD2
	style RFW fill:#FFCDD2

    click ESW "http://github.com/gnosisguild/enclave/tree/main/crates/evm/docs/EnclaveSolWriter.md"
    click EB "http://github.com/gnosisguild/enclave/tree/main/crates/events/docs/EventBus.md"
    click RFW "http://github.com/gnosisguild/enclave/tree/main/crates/evm/docs/RegistryFilterSolWriter.md"
    click EE "http://github.com/gnosisguild/enclave/tree/main/crates/events/docs/EnclaveEvent.md"
```
<details>
<summary><i>Links</i></summary>

[[EnclaveEvent]]
[[EnclaveSolWriter]]
[[EventBus]]
[[RegistryFilterSolWriter]]
</details>

```dataview
TABLE type, description as Description
FROM #evm
```
