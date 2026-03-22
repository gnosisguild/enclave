// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use tokio::io::{AsyncWrite, AsyncWriteExt};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

#[derive(Clone)]
pub struct Console {
    tx: mpsc::UnboundedSender<String>,
}
pub struct ConsoleHandle {
    console: Console,
    join: JoinHandle<()>,
}
impl Console {
    /// Output goes to stdout.
    pub fn stdout() -> ConsoleHandle {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let join = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                println!("{msg}");
            }
        });
        ConsoleHandle {
            console: Console { tx },
            join,
        }
    }

    pub fn writer(mut w: impl AsyncWrite + Unpin + Send + 'static) -> ConsoleHandle {
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        let join = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if w.write_all(msg.as_bytes()).await.is_err() {
                    break;
                }
                if w.write_all(b"\n").await.is_err() {
                    break;
                }
            }
            let _ = w.flush().await;
        });
        ConsoleHandle {
            console: Console { tx },
            join,
        }
    }

    /// Output goes to the returned receiver. Caller decides the destination.
    pub fn channel() -> (Self, mpsc::UnboundedReceiver<String>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (Self { tx }, rx)
    }

    /// Emit a message to whatever destination this context is wired to.
    pub fn log(&self, msg: String) {
        let _ = self.tx.send(msg);
    }
}

impl ConsoleHandle {
    /// Get a cheap cloneable reference to pass around.
    pub fn writer(&self) -> Console {
        self.console.clone()
    }

    /// Drop the sender and wait for the printer task to drain.
    pub async fn flush(self) {
        drop(self.console);
        let _ = self.join.await;
    }
}

#[macro_export]
macro_rules! log {
    ($ctx:expr, $($arg:tt)*) => {
        $ctx.log(format!($($arg)*))
    };
}
