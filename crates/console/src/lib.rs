use tokio::sync::mpsc;

#[derive(Clone)]
pub struct Console {
    tx: mpsc::UnboundedSender<String>,
}

impl Console {
    /// Output goes to stdout.
    pub fn stdout() -> Self {
        let (tx, mut rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                println!("{msg}");
            }
        });
        Self { tx }
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

#[macro_export]
macro_rules! log {
    ($ctx:expr, $($arg:tt)*) => {
        $ctx.log(format!($($arg)*))
    };
}
