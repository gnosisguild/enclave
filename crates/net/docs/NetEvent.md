---
type: enum
description: Base enum for all events sent down the [[NetEventChannel]]
tags:
  - net
---
## `=this.file.name`

`=this.description`


```rust
pub enum NetEvent {
    GossipData(Vec<u8>),
    GossipPublishError {
        correlation_id: CorrelationId,
        error: Arc<PublishError>,
    },
    GossipPublished {
        correlation_id: CorrelationId,
        message_id: MessageId,
    },
    KademliaPublished {
	    correlation_id: CorrelationId,
	    message_id: MessageId,
    },
    KademliaPublishError {
	    correlation_id: CorrelationId,
	    error: Arc<KademliaError>,
    },
    DialError { error: Arc<DialError> },
    ConnectionEstablished { connection_id: ConnectionId },
    OutgoingConnectionError {
        connection_id: ConnectionId,
        error: Arc<DialError>,
    },
}
```