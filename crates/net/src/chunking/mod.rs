// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

mod chunkable;
mod collector;

pub use chunkable::Chunkable;
pub use collector::{ChunkCollector, ChunkReceived};
// Re-export chunk types from events crate for convenience
pub use e3_events::{ChunkSetId, ChunkedDocument};

