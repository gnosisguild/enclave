// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::{Actor, ResponseActFuture, WrapFuture};

use anyhow::{anyhow, Result};

// Helper to allow for bail behaviour in actor model async handlers
pub fn bail<T: Actor>(a: &T) -> ResponseActFuture<T, ()> {
    Box::pin(async {}.into_actor(a))
}

pub fn bail_result<T: Actor>(a: &T, msg: impl Into<String>) -> ResponseActFuture<T, Result<()>> {
    let m: String = msg.into();
    Box::pin(async { Err(anyhow!(m)) }.into_actor(a))
}
