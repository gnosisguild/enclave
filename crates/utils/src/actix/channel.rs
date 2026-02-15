// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::{Actor, Context, Handler, Message};
use tokio::sync::{
    mpsc,
    oneshot::{self},
};

// Oneshot ==
pub struct Oneshot<M>(Option<oneshot::Sender<M>>);

impl<M> Actor for Oneshot<M>
where
    M: Message<Result = ()> + Send + 'static,
{
    type Context = actix::Context<Self>;
}

impl<M> Handler<M> for Oneshot<M>
where
    M: Message<Result = ()> + Send + 'static,
{
    type Result = ();

    fn handle(&mut self, m: M, _: &mut Context<Self>) -> Self::Result {
        self.0.take().map(|s| s.send(m));
    }
}

/// Return a oneshot channel where instead of a oneshot::Sender we use an actix::Recipient<M>.
/// After the oneshot has completed sending messages to the address will noop
pub fn oneshot<M>() -> (actix::Recipient<M>, oneshot::Receiver<M>)
where
    M: Message<Result = ()> + Send + 'static,
{
    let (tx, rx) = oneshot::channel();
    (Oneshot(Some(tx)).start().recipient(), rx)
}

// Mpsc ==

pub struct Mpsc<M>(mpsc::Sender<M>);

impl<M> Actor for Mpsc<M>
where
    M: Message<Result = ()> + Send + 'static,
{
    type Context = actix::Context<Self>;
}

impl<M> Handler<M> for Mpsc<M>
where
    M: Message<Result = ()> + Send + 'static,
{
    type Result = ();
    fn handle(&mut self, m: M, _: &mut Context<Self>) -> Self::Result {
        let s = self.0.clone();
        actix::spawn(async move {
            if let Err(e) = s.send(m).await {
                eprintln!("Failed to send message: {}", e);
            }
        });
    }
}

/// Return a mpsc channel where instead of a mpsc::Sender we use an actix::Recipient<M>.
pub fn mpsc<M>(buffer: usize) -> (actix::Recipient<M>, mpsc::Receiver<M>)
where
    M: Message<Result = ()> + Send + 'static,
{
    let (tx, rx) = mpsc::channel(buffer);
    (Mpsc(tx).start().recipient(), rx)
}
