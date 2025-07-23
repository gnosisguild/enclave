// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::{
    fmt::Display,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT_CORRELATION_ID: AtomicUsize = AtomicUsize::new(1);

/// CorrelationId provides a way to correlate commands and the events they create.
#[derive(Debug, Clone)]
pub struct CorrelationId {
    id: usize,
}

impl CorrelationId {
    pub fn new() -> Self {
        let id = NEXT_CORRELATION_ID.fetch_add(1, Ordering::SeqCst);
        Self { id }
    }
}

impl Display for CorrelationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}
