# e3-dashboard

Read-only node-operator dashboard with an embedded web UI and JSON endpoints. It projects durable
event history and network status; it does not participate in protocol decisions.

## Serve a dashboard

Construct `DashboardState::new(runtime, eventstore, aggregate_ids, network, chain_configs)` and
await `start_dashboard(port, state)` in an Actix runtime. `DashboardRuntime` contains node identity,
ports, version, and displayed chains; use the ciphernode handle's event-store reader, aggregate IDs,
and network status for the remaining runtime inputs. The CLI's
[startup integration](../cli/src/start.rs) supplies these from the running node.

Open `http://127.0.0.1:<port>/`. The server deliberately binds loopback: its API is unauthenticated
and exposes detailed operational/protocol data.

## HTTP API

- `GET /api/snapshot`: node/network overview, protocol state, operator status, E3 summaries, and
  recent events.
- `GET /api/e3?e3_id=...`: a projected request trace; unknown requests return 404.
- `GET /api/events?limit=...`: recent protocol events (up to 2,000).
- `GET /api/logs?since=...&limit=...&level=...&target=...&text=...`: operational log cursor query.
- `GET /api/updates`: cached release information.

Event projections refresh by per-aggregate sequence cursor. Operator chain status uses timed, cached
RPC calls; errors/stale diagnostic state are not canonical chain authority. Logs come from the
bounded [logger collector](../logger/README.md), separately from durable event history.

Run `cargo test --locked -p e3-dashboard` after preparing the
[EVM build prerequisites](../evm/README.md). Projection tests do not require a live public chain.
