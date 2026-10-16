# e3-console

Injectable, cloneable command output for stdout, an asynchronous writer, or a captured channel. This
lets the same CLI command print locally or return its output through the daemon. Protocol and
structured operational logging belong in [e3-logger](../logger/README.md).

## Capture command output

Inside an async Tokio context:

```rust
use e3_console::{log, Console};

let (out, mut messages) = Console::channel();
log!(out, "node {} ready", "cn1");
assert_eq!(messages.recv().await.unwrap(), "node cn1 ready");
```

`Console::stdout()` and `Console::writer(async_writer)` return a `ConsoleHandle`. Pass
`handle.writer()` to commands, drop every cloned `Console` when finished, then await
`handle.flush()` to drain the task. Holding another sender while awaiting flush prevents channel
closure and can wait indefinitely.

Writer mode adds one newline per message and flushes at closure. Channel mode returns message
strings without adding framing. Sends and writer failures are best-effort, and the queue is
unbounded: this is not acknowledged delivery or a backpressure mechanism.

The [daemon transport](../daemon-server/README.md) is a concrete output consumer.
`cargo check --locked -p e3-console --tests` checks this small crate without external services.
