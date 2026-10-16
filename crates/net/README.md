# e3-net

Ciphernode transport and network admission: libp2p QUIC/gossip, content-addressed documents, direct
request/response, and peer-history synchronization. A signed envelope authenticates a peer;
committee authorization remains with the protocol capability receiving it.

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

Run `cargo test --locked -p e3-net` from the root with
[test-helper prerequisites](../test-helpers/README.md) prepared for its test dependency graph.
