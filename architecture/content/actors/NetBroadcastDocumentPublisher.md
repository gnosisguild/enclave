---
type: actor
description: Manage publishing large documents on the DHT with coordination events
tags:
  - net
---
## `=this.file.name`

`=this.description`


Publish Flow
- Listens for signaled events eg.  [[KeygenSharesGenerated]] from the [[EventBus]]
- _Event signalling might happen by consolidating the `is_local_only()`  functionality to some kind of generic networking instruction_ #todo
- On a recognized event this component extracts the payload from the event and fires a [[NetCommand]]`::KademliaPublish` with the payload on the [[NetInterface]]
- Listen for the correlation event from [[NetEvent]]`::KademliaPublishedSuccess` and extract the CID from this event
- Take the CID and publish a [[NetCommand]]`::GossipPublish` with the `CID` and the `"notification"` content from the event. There should also be data to identify the event as a `document_published_notification` event .

Receive Flow
- Listen for [[NetEvent]]`::GossipData(data)` parse the event to detect it as a `document_published_notification` event.
- If event is a notification event then request a kademlia fetch [[NetCommand]]`::KademliaFetch` with the selected `CID`
- Once the document has been received attach it to an [[EnclaveEvent]]


#todo 
This component needs to be created

