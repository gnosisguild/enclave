// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::prelude::*;
use anyhow::Result;
use std::marker::PhantomData;
use tracing::error;

pub struct OneShotRunner<F, M>
where
    F: FnOnce(M) -> Result<()> + 'static,
    M: Message<Result = ()> + 'static,
{
    task: Option<F>,
    _marker: PhantomData<M>,
}

impl<F, M> OneShotRunner<F, M>
where
    F: FnOnce(M) -> Result<()> + 'static + Unpin,
    M: Message<Result = ()> + 'static + Unpin,
{
    pub fn new(task: F) -> Self {
        Self {
            task: Some(task),
            _marker: PhantomData,
        }
    }
    pub fn setup(task: F) -> Addr<Self> {
        Self::new(task).start()
    }
}

impl<F, M> Actor for OneShotRunner<F, M>
where
    F: FnOnce(M) -> Result<()> + 'static + Unpin,
    M: Message<Result = ()> + 'static + Unpin,
{
    type Context = Context<Self>;
}

impl<F, M> Handler<M> for OneShotRunner<F, M>
where
    F: FnOnce(M) -> Result<()> + 'static + Unpin,
    M: Message<Result = ()> + 'static + Unpin,
{
    type Result = ();

    fn handle(&mut self, msg: M, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(task) = self.task.take() {
            match task(msg) {
                Ok(_) => (),
                Err(e) => error!("{e}"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[derive(Message)]
    #[rtype(result = "()")]
    struct TestMessage(usize);

    #[actix::test]
    async fn test_one_shot_runner() {
        let call_count = Arc::new(AtomicUsize::new(0));
        let received_value = Arc::new(AtomicUsize::new(0));
        let call_count_clone = call_count.clone();
        let received_value_clone = received_value.clone();

        let runner = OneShotRunner::new(move |msg: TestMessage| {
            call_count_clone.fetch_add(1, Ordering::SeqCst);
            received_value_clone.store(msg.0, Ordering::SeqCst);
            Ok(())
        });

        let addr = runner.start();
        addr.send(TestMessage(42)).await.unwrap();
        addr.send(TestMessage(99)).await.unwrap();

        assert_eq!(received_value.load(Ordering::SeqCst), 42);
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }
}
