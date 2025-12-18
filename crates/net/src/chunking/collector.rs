// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use super::chunkable::Chunkable;
use actix::prelude::*;
use anyhow::Result;
use e3_events::{BusHandle, ChunkSetId, ChunkedDocument, E3id, EventPublisher};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

/// Collects chunks and reassembles them into complete documents
pub struct ChunkCollector<T: Chunkable> {
    /// E3 ID this collector is for
    e3_id: E3id,
    /// Chunks we've received so far, grouped by chunk_id
    received_chunks: HashMap<ChunkSetId, Vec<ChunkedDocument>>,
    /// Expected total chunks per set
    expected_totals: HashMap<ChunkSetId, u32>,
    /// When we started waiting for each chunk set (for timeout)
    start_times: HashMap<ChunkSetId, Instant>,
    /// Event bus to publish completed documents
    bus: BusHandle,
    /// Timeout duration
    timeout: Duration,
    /// Phantom data for type parameter
    _phantom: PhantomData<T>,
}

impl<T: Chunkable + Unpin + 'static> ChunkCollector<T> {
    pub fn new(e3_id: E3id, bus: BusHandle, timeout: Duration) -> Self {
        Self {
            e3_id,
            received_chunks: HashMap::new(),
            expected_totals: HashMap::new(),
            start_times: HashMap::new(),
            bus,
            timeout,
            _phantom: PhantomData,
        }
    }

    pub fn setup(e3_id: E3id, bus: BusHandle) -> Addr<Self> {
        Self::new(e3_id, bus, Duration::from_secs(60)).start()
    }

    /// Process a received chunk
    fn handle_chunk_internal(&mut self, chunk: ChunkedDocument) -> Result<Option<T>> {
        let chunk_id = chunk.chunk_id.clone();
        let total = chunk.total_chunks;

        debug!(
            "Received chunk {}/{} for chunk_id: {}",
            chunk.chunk_index + 1,
            total,
            chunk_id
        );

        // Track expected total and start time
        self.expected_totals
            .entry(chunk_id.clone())
            .or_insert(total);
        self.start_times
            .entry(chunk_id.clone())
            .or_insert_with(Instant::now);

        // Add to received chunks
        let chunks = self
            .received_chunks
            .entry(chunk_id.clone())
            .or_insert_with(Vec::new);

        // Check if we already have this chunk index
        if chunks.iter().any(|c| c.chunk_index == chunk.chunk_index) {
            debug!("Duplicate chunk {} ignored", chunk.chunk_index);
            return Ok(None);
        }

        chunks.push(chunk);

        // Check if complete
        if chunks.len() == total as usize {
            info!(
                "All {} chunks received for chunk_id: {}, reassembling...",
                total, chunk_id
            );
            let chunks = self.received_chunks.remove(&chunk_id).unwrap();
            self.expected_totals.remove(&chunk_id);
            self.start_times.remove(&chunk_id);

            let document = T::from_chunks(chunks)?;
            return Ok(Some(document));
        }

        debug!(
            "Waiting for {}/{} chunks for chunk_id: {}",
            chunks.len(),
            total,
            chunk_id
        );

        Ok(None)
    }

    /// Check for timeouts and clean up stale chunk sets
    fn check_timeouts(&mut self) {
        let now = Instant::now();
        let timed_out: Vec<_> = self
            .start_times
            .iter()
            .filter(|(_, start)| now.duration_since(**start) > self.timeout)
            .map(|(id, _)| id.clone())
            .collect();

        for chunk_id in timed_out {
            let received = self
                .received_chunks
                .get(&chunk_id)
                .map(|c| c.len())
                .unwrap_or(0);
            let expected = self.expected_totals.get(&chunk_id).copied().unwrap_or(0);

            warn!(
                "Chunk set {} timed out (received {}/{} chunks)",
                chunk_id, received, expected
            );

            self.received_chunks.remove(&chunk_id);
            self.expected_totals.remove(&chunk_id);
            self.start_times.remove(&chunk_id);
        }
    }
}

impl<T: Chunkable + Unpin + 'static> Actor for ChunkCollector<T> {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!("ChunkCollector started for E3: {}", self.e3_id);

        // Periodic timeout check every 5 seconds
        ctx.run_interval(Duration::from_secs(5), |act, _ctx| {
            act.check_timeouts();
        });
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        info!("ChunkCollector stopped for E3: {}", self.e3_id);
    }
}

/// Message to send a chunk to the collector
#[derive(Message, Clone, Debug)]
#[rtype(result = "()")]
pub struct ChunkReceived<T: Chunkable> {
    pub chunk: ChunkedDocument,
    pub _phantom: PhantomData<T>,
}

impl<T: Chunkable> ChunkReceived<T> {
    pub fn new(chunk: ChunkedDocument) -> Self {
        Self {
            chunk,
            _phantom: PhantomData,
        }
    }
}

impl<T: Chunkable + Unpin + 'static> Handler<ChunkReceived<T>> for ChunkCollector<T>
where
    T: actix::Message + Send + Into<e3_events::EnclaveEventData>,
    T::Result: Send,
{
    type Result = ();

    fn handle(&mut self, msg: ChunkReceived<T>, _ctx: &mut Self::Context) {
        match self.handle_chunk_internal(msg.chunk) {
            Ok(Some(document)) => {
                info!("Document reassembled successfully, publishing to bus");
                // Publish the reassembled document to the event bus
                if let Err(e) = self.bus.publish(document) {
                    error!("Failed to publish reassembled document: {:?}", e);
                }
            }
            Ok(None) => {
                // Still waiting for more chunks
            }
            Err(e) => {
                error!("Failed to process chunk: {:?}", e);
            }
        }
    }
}

// Note: ChunkCollector tests are covered by integration tests with real event types
// that implement Into<EnclaveEventData>. Unit tests with mock types would require
// duplicating the handler implementation without the publish constraint.
