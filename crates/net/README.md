# e3-net

## Purpose

Ciphernode transport and network admission: libp2p QUIC/gossip, content-addressed documents, direct
request/response, and peer-history synchronization. A signed envelope authenticates a peer;
committee authorization remains with the protocol capability receiving it.

## Map

| Location                                                                                                                                                         | Responsibility                                                                        |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| [`lib.rs`](src/lib.rs), [`network.rs`](src/network.rs)                                                                                                           | Runtime setup and network/deployment protocol policy.                                 |
| [`net_interface.rs`](src/net_interface.rs), [`net_interface_handle.rs`](src/net_interface_handle.rs)                                                             | libp2p swarm and command/event channel boundary.                                      |
| [`peer_admission.rs`](src/peer_admission.rs), [`gossip_ingress.rs`](src/gossip_ingress.rs), [`gossip_subscription_health.rs`](src/gossip_subscription_health.rs) | Peer compatibility, bounded ingress, and topic readiness/repair.                      |
| [`document_publishing/`](src/document_publishing)                                                                                                                | Content-addressed upload/fetch, announcements, recovery, and participation interests. |
| [`event_buffer/`](src/event_buffer), [`event_conversion/`](src/event_conversion), [`event_translation/`](src/event_translation)                                  | Startup buffering and translation between network and domain events.                  |
| [`network_sync/`](src/network_sync), [`live_history.rs`](src/live_history.rs)                                                                                    | Bounded peer-history exchange and live coverage tracking.                             |
| [`direct_requester.rs`](src/direct_requester.rs), [`direct_responder.rs`](src/direct_responder.rs)                                                               | Correlated peer request/response API.                                                 |

The swarm admits compatible peers and separates control/command results from application ingress.
Startup gathers history while application events wait in a bounded buffer. Once sync ends, the
translator and document layer deliver admitted content to the bus; outbound protocol events take the
reverse conversion/publication path.

## Connect a runtime

`setup_libp2p_keypair(repository, cipher)` loads the encrypted peer identity and fails if missing.
`NetworkPolicy::new(profile, deployments)` binds the network ID and chain/deployment admission
rules. `local_unrestricted()` is an explicit local/test policy, not a public-network default.

`setup_net_interface(policy, keypair, peers, quic_port, max_buffered_events)` starts the transport;
`setup_net` connects a `NetInterface` to the event bus and timestamp-history reader. The
`setup_net_with_limits` / `setup_net_with_limits_and_interests` variants configure startup buffering
and participation-aware document fetching. See
[builder network startup](../ciphernode-builder/src/builder/net.rs) for the complete recipe,
including readiness and the transport task. Normal node users use
[CiphernodeBuilder](../ciphernode-builder/README.md).

For an in-process harness, `create_channel_bridge()` returns a `NetInterfaceHandle` and inverted
`NetChannelBridge`; [test helpers](../test-helpers/README.md) provide a mock network. These model
transport, not proof validity or membership authorization.

## Other consumers

`direct_requester::DirectRequester::builder(commands, events)` configures retries/deadlines; select
a peer with `to(...)` before `request(...)`. `DirectResponder` replies to incoming requests.
`NetworkStatus::snapshot()` supplies connected-peer/topic/error diagnostics to the dashboard.
Connection count alone does not establish gossip readiness.

Documents are content-checked and fetched with bounded work; DHT acknowledgments are not durable
replication. Publications wait for initial synchronization, while registered command responses
remain deliverable during startup. See
[network delivery](../../docs/architecture/network-delivery.md) for policies and loss windows.

## Tests

Run `cargo test --locked -p e3-net` from the root with
[test-helper prerequisites](../test-helpers/README.md) prepared for its test dependency graph.

Capability-local tests cover document admission/recovery, bounded buffering, and history paging.
[`net_interface_swarm_tests.rs`](src/net_interface_swarm_tests.rs) exercises swarm behavior;
[`tests/`](tests) holds peer-reconnect, identity-mismatch, and network/deployment isolation tests
using local peers. [AGENTS.md](AGENTS.md) carries transport/admission change constraints.
