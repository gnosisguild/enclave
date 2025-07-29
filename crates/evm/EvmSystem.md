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

    click ESR "https://github.com/gnosisguild/enclave/tree/main/crates/evm/docs/EnclaveSolReader.md"
    click ER "https://github.com/gnosisguild/enclave/tree/main/crates/evm/docs/EventReader.md"
    click CRR "https://github.com/gnosisguild/enclave/tree/main/crates/evm/docs/CiphernodeRegistryReader.md"
    click EB "https://github.com/gnosisguild/enclave/tree/main/crates/events/docs/EventBus.md"
    click EE "https://github.com/gnosisguild/enclave/tree/main/crates/events/docs/EnclaveEvent.md"
```
<details>
<summary>Links</summary>

[[CiphernodeRegistryReader]]
[[EnclaveEvent]]
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

    click ESW "https://github.com/gnosisguild/enclave/tree/main/crates/evm/docs/EnclaveSolWriter.md"
    click EB "https://github.com/gnosisguild/enclave/tree/main/crates/events/docs/EventBus.md"
    click RFW "https://github.com/gnosisguild/enclave/tree/main/crates/evm/docs/RegistryFilterSolWriter.md"
    click EE "https://github.com/gnosisguild/enclave/tree/main/crates/events/docs/EnclaveEvent.md"
```
<details>
<summary>Links</summary>

[[EnclaveEvent]]
[[EnclaveSolWriter]]
[[EventBus]]
[[RegistryFilterSolWriter]]
</details>

```dataview
TABLE type, description as Description
FROM #evm
```
