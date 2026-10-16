# e3-daemon-server

Loopback command transport between the local CLI and an already-running node. The crate transports
serialized commands and text output; callers own command authorization and execution.

## Client and server

- `connect_daemon(Option<&AppConfig>).await` probes the configured control port and returns
  `Option<ServerInfo>`. Reachability is not authenticated daemon identity.
- `run_on_daemon(out, server, command).await` POSTs a serializable command, propagates HTTP
  failures, and writes response lines to an `e3_console::Console`.
- `start_daemon_server(port, handler).await` binds `127.0.0.1` and passes each request body's string
  to an asynchronous handler returning `anyhow::Result<String>`.

The server accepts non-Send handlers and uses `spawn_local`: run it inside an Actix/local-task
context, not an ordinary multi-thread Tokio task alone. The CLI's
[socket setup](../cli/src/start.rs) demonstrates the complete local command handler; allowed remote
command conversions are in [cli.rs](../cli/src/cli.rs).

## Transport limits

There is no authentication or TLS. Keep the control socket local. Its manual HTTP parser supports
content-length framing, not arbitrary chunked requests, and is not a general-purpose HTTP server
with framework body/time limits. Handler errors become HTTP 500 responses.

Use [program-server](../program-server/README.md) for the separate computation-job HTTP API.
`cargo check --locked -p e3-daemon-server --tests` checks this crate; no live ciphernode is needed
for transport-only work.
