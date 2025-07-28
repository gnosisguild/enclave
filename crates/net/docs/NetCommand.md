---
type: enum
description: Enum base for all commands sent to the [[NetCommandChannel]]
tags:
  - net
---
## `=this.file.name`

`=this.description`


```rust
pub enum NetCommand {
    GossipPublish {
        topic: String,
        data: Vec<u8>,
        correlation_id: CorrelationId,
    },
    KademliaPublish {
	    data: Vec<u8>,
	    correlation_id: CorrelationId
    },
    Dial(DialOpts),
}
```
