// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::{Actor, Handler, Message, Recipient};
use e3_events::CommitSnapshot;

use crate::{Insert, InsertBatch};

pub struct WriteBuffer {
    dest: Option<Recipient<InsertBatch>>,
    buffer: Vec<Insert>,
}

impl Actor for WriteBuffer {
    type Context = actix::Context<Self>;
}

impl WriteBuffer {
    /// Creates a new WriteBuffer with no destination and an empty buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// let wb = crate::write_buffer::WriteBuffer::new();
    /// assert!(wb.dest.is_none());
    /// assert!(wb.buffer.is_empty());
    /// ```
    ///
    /// # Returns
    ///
    /// A `WriteBuffer` with `dest` set to `None` and an empty `buffer`.
    pub fn new() -> Self {
        Self {
            dest: None,
            buffer: Vec::new(),
        }
    }
}

impl Handler<ForwardTo> for WriteBuffer {
    type Result = ();
    /// Sets the destination recipient for future insert batches.
    ///
    /// This handler updates the write buffer's internal destination so subsequent
    /// commit operations will forward buffered `InsertBatch` messages to that
    /// recipient.
    ///
    /// # Parameters
    ///
    /// * `msg` - A `ForwardTo` message containing the `Recipient<InsertBatch>` to use.
    fn handle(&mut self, msg: ForwardTo, _: &mut Self::Context) -> Self::Result {
        self.dest = Some(msg.dest())
    }
}

impl Handler<Insert> for WriteBuffer {
    type Result = ();

    /// Enqueues an `Insert` message into the actor's in-memory buffer for later batching.
    ///
    /// This does not send or commit the insert; buffered items are forwarded when a `CommitSnapshot` is handled.
    ///
    /// # Examples
    ///
    /// ```
    /// // Illustrative example — actual `Insert` constructor may differ in your codebase.
    /// let mut wb = WriteBuffer::new();
    /// let insert = Insert::new("key", b"value".to_vec());
    /// wb.handle(insert, &mut wb_actor_context()); // after this, `wb.buffer` contains the insert
    /// ```
    fn handle(&mut self, msg: Insert, _: &mut Self::Context) -> Self::Result {
        self.buffer.push(msg);
    }
}

impl Handler<CommitSnapshot> for WriteBuffer {
    type Result = ();

    /// Flushes buffered inserts on a commit by sending them as an InsertBatch to the configured destination.
    ///
    /// If a destination recipient is set and the buffer contains any `Insert` items, this handler:
    /// - takes the current buffer contents,
    /// - appends an `Insert` with key `"//seq"` and the commit sequence encoded as big-endian bytes,
    /// - constructs an `InsertBatch` from those inserts,
    /// - sends the batch to the destination, and
    /// - leaves the internal buffer empty.
    ///
    /// The handler does nothing if no destination is configured or if the buffer is empty.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Example (illustrative):
    /// // let mut wb = WriteBuffer::new();
    /// // wb.handle(ForwardTo::new(dest_recipient), &mut ctx);
    /// // wb.handle(Insert::new("k", b"v".to_vec()), &mut ctx);
    /// // wb.handle(CommitSnapshot::new(42), &mut ctx);
    /// // // dest_recipient receives an InsertBatch containing the buffered insert and a "//seq" entry with 42.
    /// ```
    fn handle(&mut self, msg: CommitSnapshot, _: &mut Self::Context) -> Self::Result {
        if let Some(ref dest) = self.dest {
            if !self.buffer.is_empty() {
                let mut inserts = std::mem::take(&mut self.buffer);
                inserts.push(Insert::new("//seq", msg.seq().to_be_bytes().to_vec()));
                let batch = InsertBatch::new(inserts);
                dest.do_send(batch);
            }
        }
    }
}

#[derive(Message)]
#[rtype("()")]
pub struct ForwardTo(Recipient<InsertBatch>);

impl ForwardTo {
    /// Creates a `ForwardTo` wrapper that holds a recipient for `InsertBatch` messages.
    ///
    /// The function accepts any value that can be converted into `Recipient<InsertBatch>` and
    /// stores the converted recipient inside the returned `ForwardTo`.
    ///
    /// # Examples
    ///
    /// ```
    /// // `recipient` should be a `Recipient<InsertBatch>` obtained from an Actix actor.
    /// let recipient: Recipient<InsertBatch> = /* obtain recipient */;
    /// let forward = ForwardTo::new(recipient);
    /// ```
    pub fn new(dest: impl Into<Recipient<InsertBatch>>) -> Self {
        Self(dest.into())
    }

    /// Accesses the contained `Recipient<InsertBatch>`.
    ///
    /// # Examples
    ///
    /// ```
    /// // obtain or construct a Recipient<InsertBatch> in your application context
    /// let recipient: Recipient<InsertBatch> = /* ... */;
    /// let fwd = ForwardTo::new(recipient);
    /// let dest = fwd.dest();
    /// ```
    pub fn dest(self) -> Recipient<InsertBatch> {
        self.0
    }
}