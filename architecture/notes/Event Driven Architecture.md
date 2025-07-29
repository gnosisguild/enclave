We use an event-driven architecture because peer-to-peer systems are built around events. Everything in a P2P network happens through events. Our architecture mirrors this reality.

We get events from two main sources: the EVM (blockchain events like transactions and state changes) and libp2p (network events like peer connections and message routing). We put all these events on a central event bus. Different parts of our system listen to this bus and handle the events they need.

This works well because our components stay independent. The blockchain handlers don't need to know about network details, and the network handlers don't need to know about blockchain state. When we need to connect blockchain events with network events, we can do it easily since everything goes through the same bus.

This architecture scales well and keeps our code maintainable and focused.
