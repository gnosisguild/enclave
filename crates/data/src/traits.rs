// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::Result;

use crate::{Get, Insert, Remove, SeekForPrev};

pub trait KeyValStore {
    fn insert(&mut self, msg: Insert) -> Result<()>;
    fn remove(&mut self, msg: Remove) -> Result<()>;
    fn get(&self, msg: Get) -> Result<Option<Vec<u8>>>;
}

pub trait SeekableStore: KeyValStore {
    /// Seek for the first key that is less than or equal to the given key in the SeekForPrev msg
    /// and return the value as a Some variant. If no value exists return None. If there was an
    /// error doing the seek return an Error.
    fn seek_for_prev(&self, msg: SeekForPrev) -> Result<Option<Vec<u8>>>;
}
