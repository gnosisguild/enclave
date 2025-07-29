---
type: system
description: Manages sending and receiving messages to and from from the network using libp2p
tags:
  - net
---

## `=this.file.name`

`=this.description`


Here we separate command and query according to the principle of [CQRS](https://cqrs.wordpress.com/wp-content/uploads/2010/11/cqrs_documents.pdf): 

```mermaid
flowchart TB
    subgraph s1["Network Events Received"]
        NET["NetEventTranslator"]
        NBDP["NetDHTPublisher"]
        EB["EventBus"]
        NEC["NetEventChannel"]
        NI["NetInterface"]
        FC["Future Component"]
    end
    NI --> NEC
    NEC --> NET
    NET --> EB
    NBDP --> EB
    NEC --> NBDP
    NEC --> FC
    
    NEC@{ shape: h-cyl}

    style FC fill:#CCCCCC,stroke-width:2px,stroke-dasharray: 2,stroke:#CCCCCC
	NET:::internal-link
	NBDP:::internal-link
	EB:::internal-link
	NEC:::internal-link
	NI:::internal-link

    click NET "http://github.com/gnosisguild/enclave/tree/main/crates/net/docs/NetEventTranslator.md"
    click NBDP "http://github.com/gnosisguild/enclave/tree/main/crates/net/docs/NetDHTPublisher.md"
    click EB "http://github.com/gnosisguild/enclave/tree/main/crates/events/docs/EventBus.md"
    click NEC "http://github.com/gnosisguild/enclave/tree/main/crates/net/docs/NetEventChannel.md"
    click NI "http://github.com/gnosisguild/enclave/tree/main/crates/net/docs/NetInterface.md"
```
<details>
<summary>Links</summary>

[[EventBus]]
[[NetDHTPublisher]]
[[NetEventChannel]]
[[NetEventTranslator]]
[[NetInterface]]
</details>

```mermaid
flowchart TB
    subgraph s1["Network Commands Sent"]
        EB["EventBus"]
        NET["NetEventTranslator"]
        NBDP["NetDHTPublisher"]
        NCC["NetCommandChannel"]
        NI["NetInterface"]
        FC["Future Component"]
    end
    EB --> NET & NBDP
    NET --> NCC
    NCC --> NI
    NBDP --> NCC
    FC --> NCC

    NCC@{ shape: h-cyl}

    style FC fill:#CCCCCC,stroke-width:2px,stroke-dasharray: 2,stroke:#CCCCCC
	
	EB:::internal-link
	NET:::internal-link
	NBDP:::internal-link
	NCC:::internal-link
	NI:::internal-link

    click EB "http://github.com/gnosisguild/enclave/tree/main/crates/events/docs/EventBus.md"
    click NET "http://github.com/gnosisguild/enclave/tree/main/crates/net/docs/NetEventTranslator.md"
    click NBDP "http://github.com/gnosisguild/enclave/tree/main/crates/net/docs/NetDHTPublisher.md"
    click NCC "http://github.com/gnosisguild/enclave/tree/main/crates/net/docs/NetCommandChannel.md"
    click NI "http://github.com/gnosisguild/enclave/tree/main/crates/net/docs/NetInterface.md"
```
<details>
<summary>Links</summary>

[[EventBus]]
[[NetCommandChannel]]
[[NetDHTPublisher]]
[[NetEventTranslator]]
[[NetInterface]]
</details>


This we can easily extend to a future networking component by listening to the [[EventBus]] and sending to the [[NetCommandChannel]] or reading from the [[NetEventChannel]] and publishing to the [[EventBus]]

```dataview
TABLE type, description as Description
FROM #net
```
### Benefits

- Extensive: Can use channels to control the network interface from multiple actors.
- Separates Domain Logic (EventBus) from lower level implementation (Net System)
- [[NetInterface]] should remain a dumb component that exposes libp2p functionality and can be called using channel commands.