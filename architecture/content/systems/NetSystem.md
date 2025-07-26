---
type: system
description: Coordinates sending and receiving messages from the network (libp2p) interface
tags:
  - net
---
## `=this.file.name`

`=this.description`

```mermaid
flowchart TB
    subgraph s1["Net System"]
        NET["NetEventTranslator"]
        NBDP["NetBroadcastDocumentPublisher"]
        EB["EventBus"]
        NCC["NetCommandChannel"]
        NEC["NetEventChannel"]
        NI["NetInterface"]
        FC["Future Component"]
    end
    EB <--> NET & NBDP
    NET --> NCC
    NET -.- NEC
    NCC --> NI
    NEC -.- NI
    NBDP --> NCC
    NBDP -.- NEC
    FC -.- NEC
    FC --> NCC

    NCC@{ shape: h-cyl}
    NEC@{ shape: h-cyl}
    style NET fill:#FFCDD2
    style NBDP fill:#FFCDD2
    style EB fill:#FFCDD2
    style NCC stroke-width:1px,stroke-dasharray: 0,fill:#BBDEFB
    style NEC stroke-width:1px,stroke-dasharray: 0,fill:#BBDEFB
    style NI stroke-width:1px,stroke-dasharray: 0,fill:#C8E6C9
    style FC fill:#CCCCCC,stroke-width:2px,stroke-dasharray: 2,stroke:#CCCCCC
	class EB internal-link
	class NET internal-link
	class NI internal-link
	class NCC internal-link
	class NEC internal-link
	class NBDP internal-link
    linkStyle 8 stroke:#CCCCCC,fill:none
    linkStyle 9 stroke:#CCCCCC,fill:none
```

```dataview
TABLE description as Description
FROM #net
```
### Benefits

- Extensive: Can use channels to control the network interface from multiple actors.
- Separates Domain Logic (EventBus) from lower level implementation (Net System)
- [[NetInterface]] is a dumb component that exposes libp2p functionality and can be called using channel commands.