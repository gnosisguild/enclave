// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::{bail, Result};
use e3_events::{ChunkSetId, ChunkedDocument};
use serde::{de::DeserializeOwned, Serialize};
use tracing::{debug, info};

/// Trait for documents that can be split into chunks for transmission
pub trait Chunkable: Serialize + DeserializeOwned + Clone + Sized {
    fn max_chunk_size() -> usize {
        10 * 1024 * 1024
    }

    /// Mark this document as received from external source (network).
    /// Called after reassembly to prevent re-publication loops.
    /// Default implementation does nothing - override for types with external flag.
    fn mark_as_external(&mut self) {}

    fn into_chunks(&self) -> Result<Vec<ChunkedDocument>> {
        let bytes = bincode::serialize(self)?;
        let max_size = Self::max_chunk_size();

        debug!(
            "Chunking document: {} bytes, max chunk size: {} bytes",
            bytes.len(),
            max_size
        );

        if bytes.len() <= max_size {
            // Small enough, send as single chunk
            debug!("Document fits in single chunk");
            return Ok(vec![ChunkedDocument::single(bytes)]);
        }

        // Split into multiple chunks
        let num_chunks = (bytes.len() + max_size - 1) / max_size;
        let chunk_id = ChunkSetId::from_content(&bytes);

        info!(
            "Splitting document into {} chunks (chunk_id: {})",
            num_chunks, chunk_id
        );

        Ok(bytes
            .chunks(max_size)
            .enumerate()
            .map(|(idx, chunk)| {
                ChunkedDocument::new(
                    chunk_id.clone(),
                    idx as u32,
                    num_chunks as u32,
                    chunk.to_vec(),
                )
            })
            .collect())
    }

    /// Reassemble from chunks
    fn from_chunks(chunks: Vec<ChunkedDocument>) -> Result<Self> {
        if chunks.is_empty() {
            bail!("Cannot reassemble from zero chunks");
        }

        // If single chunk, just deserialize
        if chunks.len() == 1 && chunks[0].is_complete_set() {
            return Ok(bincode::deserialize(&chunks[0].data)?);
        }

        // Validate all chunks are from same set
        let chunk_id = &chunks[0].chunk_id;
        let total = chunks[0].total_chunks;

        if chunks.len() != total as usize {
            bail!("Missing chunks: got {} expected {}", chunks.len(), total);
        }

        if !chunks.iter().all(|c| c.chunk_id == *chunk_id) {
            bail!("Chunks from different sets");
        }

        // Check for duplicate indices
        let mut indices: Vec<u32> = chunks.iter().map(|c| c.chunk_index).collect();
        indices.sort_unstable();
        indices.dedup();
        if indices.len() != chunks.len() {
            bail!("Duplicate chunk indices detected");
        }

        // Sort by index and concatenate
        let mut sorted = chunks;
        sorted.sort_by_key(|c| c.chunk_index);

        let bytes: Vec<u8> = sorted.into_iter().flat_map(|c| c.data).collect();

        info!(
            "Reassembling document from {} chunks ({} bytes total)",
            total,
            bytes.len()
        );

        Ok(bincode::deserialize(&bytes)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
    struct TestDoc {
        data: Vec<u8>,
    }

    impl Chunkable for TestDoc {
        fn max_chunk_size() -> usize {
            100
        }
    }

    #[test]
    fn test_small_document_single_chunk() {
        let doc = TestDoc {
            data: vec![1, 2, 3, 4, 5],
        };
        let chunks = doc.into_chunks().unwrap();
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].is_complete_set());

        let restored = TestDoc::from_chunks(chunks).unwrap();
        assert_eq!(doc, restored);
    }

    #[test]
    fn test_large_document_multiple_chunks() {
        let doc = TestDoc {
            data: vec![42; 500],
        };
        let chunks = doc.into_chunks().unwrap();
        assert!(chunks.len() > 1);

        // Verify all chunks have same chunk_id
        let first_id = &chunks[0].chunk_id;
        assert!(chunks.iter().all(|c| c.chunk_id == *first_id));

        // Verify chunk indices
        for (idx, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.chunk_index, idx as u32);
            assert_eq!(chunk.total_chunks, chunks.len() as u32);
        }

        let restored = TestDoc::from_chunks(chunks).unwrap();
        assert_eq!(doc, restored);
    }

    #[test]
    fn test_missing_chunk_fails() {
        let doc = TestDoc {
            data: vec![42; 500],
        };
        let mut chunks = doc.into_chunks().unwrap();
        chunks.pop(); // Remove last chunk

        let result = TestDoc::from_chunks(chunks);
        assert!(result.is_err());
    }

    #[test]
    fn test_mixed_chunk_sets_fails() {
        let doc1 = TestDoc {
            data: vec![42; 500],
        };
        let doc2 = TestDoc {
            data: vec![99; 500],
        };

        let mut chunks1 = doc1.into_chunks().unwrap();
        let chunks2 = doc2.into_chunks().unwrap();

        // Mix chunks from different sets
        chunks1[0] = chunks2[0].clone();

        let result = TestDoc::from_chunks(chunks1);
        assert!(result.is_err());
    }
}
