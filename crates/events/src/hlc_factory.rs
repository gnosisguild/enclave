// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::sync::{Arc, Mutex, PoisonError};

use thiserror::Error;

use crate::hlc::{Hlc, HlcError, HlcMethods, HlcTimestamp};

#[derive(Debug, Error)]
pub enum HlcFactoryError {
    #[error(transparent)]
    Hlc(#[from] HlcError),

    #[error("HLC not initialized")]
    NotReady,

    #[error("lock poisoned: {0}")]
    LockPoisoned(String),
}

impl<T> From<PoisonError<T>> for HlcFactoryError {
    fn from(e: PoisonError<T>) -> Self {
        HlcFactoryError::LockPoisoned(e.to_string())
    }
}

enum HlcState {
    Init,
    Ready(Hlc),
}

impl HlcState {
    pub fn ready(&mut self, hlc: Hlc) {
        if let HlcState::Init = self {
            *self = HlcState::Ready(hlc);
        }
    }

    pub fn is_ready(&self) -> bool {
        if let HlcState::Init = self {
            false
        } else {
            true
        }
    }
}

impl HlcMethods for HlcState {
    type Error = HlcFactoryError;
    fn receive(&self, remote: &HlcTimestamp) -> Result<HlcTimestamp, Self::Error> {
        let HlcState::Ready(hlc) = self else {
            return Err(HlcFactoryError::NotReady);
        };

        Ok(hlc.receive(remote)?)
    }
    fn tick(&self) -> Result<HlcTimestamp, Self::Error> {
        let HlcState::Ready(hlc) = self else {
            return Err(HlcFactoryError::NotReady);
        };

        Ok(hlc.tick()?)
    }
}

/// This solves an issue where Hlc needs a node_id which is derived from the address but the
/// address is located in the store but the store needs the handle but the handle needs the hlc
/// which needs the node_id.
#[derive(Clone)]
pub struct HlcFactory {
    hlc: Arc<Mutex<HlcState>>,
}

impl HlcFactory {
    pub fn new() -> Self {
        Self {
            hlc: Arc::new(Mutex::new(HlcState::Init)),
        }
    }

    pub fn enable(&self, hlc: Hlc) {
        let mut guard = self.hlc.lock().unwrap();
        guard.ready(hlc);
    }

    pub fn is_ready(&self) -> bool {
        match self.hlc.lock() {
            Err(_) => false,
            Ok(g) => g.is_ready(),
        }
    }
}

impl HlcMethods for HlcFactory {
    type Error = HlcFactoryError;
    fn tick(&self) -> Result<HlcTimestamp, HlcFactoryError> {
        let guard = self.hlc.lock()?;
        guard.tick()
    }
    fn receive(&self, remote: &HlcTimestamp) -> Result<HlcTimestamp, HlcFactoryError> {
        let guard = self.hlc.lock()?;
        guard.receive(remote)
    }
}
