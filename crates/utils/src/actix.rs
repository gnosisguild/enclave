// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::ops::Deref;

use actix::{dev::SendError, Actor, Message, Recipient, ResponseActFuture, WrapFuture};

use anyhow::{anyhow, Result};

// Helper to allow for bail behaviour in actor model async handlers
pub fn bail<T: Actor>(a: &T) -> ResponseActFuture<T, ()> {
    Box::pin(async {}.into_actor(a))
}

pub fn bail_result<T: Actor>(a: &T, msg: impl Into<String>) -> ResponseActFuture<T, Result<()>> {
    let m: String = msg.into();
    Box::pin(async { Err(anyhow!(m)) }.into_actor(a))
}

#[derive(Message)]
#[rtype("()")]
pub struct Responder<T, U: Send>
where
    U: Message + Send,
    U::Result: Send,
{
    value: T,
    sender: Recipient<U>,
}

impl<T, U> Responder<T, U>
where
    U: Message + Send,
    U::Result: Send,
{
    pub fn new(value: T, sender: impl Into<Recipient<U>>) -> Self {
        Self {
            value,
            sender: sender.into(),
        }
    }

    pub fn reply(&self, msg: U) {
        let sender = &self.sender;
        sender.do_send(msg);
    }

    pub fn try_reply(&self, msg: U) -> Result<(), SendError<U>> {
        let sender = &self.sender;
        sender.try_send(msg)
    }
}

impl<T, U> Deref for Responder<T, U>
where
    U: Message + Send,
    U::Result: Send,
{
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
