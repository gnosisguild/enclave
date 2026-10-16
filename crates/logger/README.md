# e3-logger

Passive protocol summaries and bounded operational diagnostics. Durable protocol history stays in
the EventStore; this logger does not provide replay or acknowledged event delivery.

## Attach logging

`attach_protocol_logger(name, bus)` subscribes a `SimpleLogger<InterfoldEvent>` to summarize
protocol events. `EventLogging` supports other event types. For operational `tracing` output,
initialize `LogCollector::init(node_name, optional_jsonl_path)` and add `OperationalLogLayer` to the
application's tracing subscriber. The CLI's [telemetry setup](../cli/src/helpers/telemetry.rs)
demonstrates subscriber/OpenTelemetry wiring.

The collector is process-global and keeps the first initialization. It retains up to 20,000 entries
in memory; optional JSONL output rotates at 25 MiB with four rotated files. `LogCollector::global()`
retrieves the initialized instance and `flush()` flushes its writer. Failure to open the diagnostic
file does not create protocol authority or replace the event log.

## Query and privacy

`query(since, limit, level, target, text)` returns `LogQueryResult` with entries, next/oldest
cursors, and total retained count. Cursors are collector sequence numbers, not durable event
sequences; older entries may have been evicted. The [dashboard](../dashboard/README.md) exposes this
via `/api/logs` separately from its EventStore-backed protocol view.

Field-name-based redaction filters sensitive fields and inherited spans. OpenTelemetry has its own
metadata filter; one sibling tracing layer cannot sanitize another layer's output. Neither filter
can make arbitrary secret text in a message safe. Do not log raw secrets, plaintext, credentials, or
entire cryptographic payloads.

For CLI verbosity and collector endpoints see the
[workspace debugging guide](../README.md#debugging). Run `cargo test --locked -p e3-logger` for
redaction, pagination, and file rotation coverage.
