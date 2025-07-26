---
type: system
description: Coordinates sending and receiving messages from the network (libp2p) interface
---
## `=this.file.name`

`=this.description`

```mermaid
flowchart TB
    subgraph s1["Net System"]
        NET["NetEventTranslater"]
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

### Description
- **[[EventBus]]** is the central application event bus
- **[[NetCommandChannel]]** is an mpsc channel for sending `NetCommand`s to the `NetInterface`
- **[[NetEventChannel]]** is an broadcast channel for broadcasting `NetEvent`s from the `NetInterface`
- **[[NetEventTranslator]]** works bidirectionally converting `EnclaveEvent`s to the appropriate `NetCommand` for the network peer and `NetEvent`s to the appropriate `EnclaveEvent`s
- **[[NetBroadcastDocumentPublisher]]** listens for specific `PublishDocumentRequested` events and will send appropriate events to the `NetInterface` in order to publish the document payload to the DHT as well as publish a libp2p gossipsub `DocumentPublished` event on the `NetInterface`
- **[[NetInterface]]** exposes two channels for control: `NetEventChannel` and `NetCommandChannel` 
### Benefits

- Extensive: Can use channels to control the network interface from multiple actors.
- Separates Domain Logic (EventBus) from lower level implementation (Net System)
- [[NetInterface]] is a dumb component that exposes libp2p functionality and can be called using channel commands.