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
    style NET fill:#FFCDD2
    style NBDP fill:#FFCDD2
    style EB fill:#FFCDD2
    style NEC stroke-width:1px,stroke-dasharray: 0,fill:#BBDEFB
    style NI stroke-width:1px,stroke-dasharray: 0,fill:#C8E6C9
    style FC fill:#CCCCCC,stroke-width:2px,stroke-dasharray: 2,stroke:#CCCCCC
	class EB internal-link
	class NET internal-link
	class NI internal-link
	class NCC internal-link
	class NEC internal-link
	class NBDP internal-link
```
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
    style NET fill:#FFCDD2
    style NBDP fill:#FFCDD2
    style EB fill:#FFCDD2
    style NCC stroke-width:1px,stroke-dasharray: 0,fill:#BBDEFB
    style NI stroke-width:1px,stroke-dasharray: 0,fill:#C8E6C9
    style FC fill:#CCCCCC,stroke-width:2px,stroke-dasharray: 2,stroke:#CCCCCC
	class EB internal-link
	class NET internal-link
	class NI internal-link
	class NCC internal-link
	class NEC internal-link
	class NBDP internal-link
```

This we can easily extend to a future networking component by listening to the [[EventBus]] and sending to the [[NetCommandChannel]] or reading from the [[NetEventChannel]] and publishing to the [[EventBus]]

```dataview
TABLE type, description as Description
FROM #net
```
### Benefits

- Extensive: Can use channels to control the network interface from multiple actors.
- Separates Domain Logic (EventBus) from lower level implementation (Net System)
- [[NetInterface]] should remain a dumb component that exposes libp2p functionality and can be called using channel commands.