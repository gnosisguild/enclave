---
type: actor
description: Listens for KeygenSharesGenerated events and publishes the payload to kademlia whilst simultaneously dispatching a gossipsub event KeygenSharesPublished
---

## `=this.file.name`

`=this.description`

This actor lives in the `net` package and is responsible for handling DHT related behaviour
It listens for [[KeygenSharesGenerated]] and then sends the appropriate [[NetworkPeerCommand]] via mpsc channel to the [[NetworkPeer]] to publish the appropriate documents on kademlia