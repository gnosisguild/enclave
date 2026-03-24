// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Describes the public output (return value) layout of each ZK circuit.
//!
//! In Noir, circuits declare `pub` input parameters and `-> pub` return values.
//! Both end up in the proof's `public_signals` byte array, with return values
//! placed **after** all public inputs. This module provides the metadata needed
//! to extract named return fields from a proof's public signals without
//! hard-coding byte offsets.

use serde::{Deserialize, Serialize};

/// Size of a single Noir `Field` element in bytes (BN254 scalar).
pub const FIELD_BYTE_LEN: usize = 32;

/// A named output field of a circuit proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OutputField {
    /// Human-readable name (e.g. `"pk_commitment"`).
    pub name: &'static str,
}

/// Describes the public return values of a circuit.
///
/// `fields` lists them in the order they appear in `public_signals`,
/// which is the same order as the Noir `-> pub (A, B, C)` tuple.
///
/// Circuits whose output count depends on runtime parameters (e.g.
/// `SkShareComputationBase` whose return is `[[Field; L]; N]`)
/// use [`CircuitOutputLayout::Dynamic`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CircuitOutputLayout {
    /// Fixed number of `Field`-sized outputs, names known at compile time.
    Fixed { fields: &'static [OutputField] },
    /// The circuit returns no public values (void).
    None,
    /// Output count depends on runtime parameters — callers must supply the
    /// element count themselves.
    Dynamic,
}

impl CircuitOutputLayout {
    /// Number of fixed output fields, or `None` for dynamic / void layouts.
    pub fn field_count(&self) -> Option<usize> {
        match self {
            CircuitOutputLayout::Fixed { fields } => Some(fields.len()),
            CircuitOutputLayout::None => Some(0),
            CircuitOutputLayout::Dynamic => None,
        }
    }

    /// Look up a field index by name.
    pub fn field_index(&self, name: &str) -> Option<usize> {
        match self {
            CircuitOutputLayout::Fixed { fields } => fields.iter().position(|f| f.name == name),
            _ => None,
        }
    }

    /// Extract a named output field from raw `public_signals` bytes.
    ///
    /// Return values sit at the **end** of `public_signals`, after any
    /// `pub` input parameters. This method indexes from the tail.
    pub fn extract_field<'a>(&self, public_signals: &'a [u8], name: &str) -> Option<&'a [u8]> {
        let fields = match self {
            CircuitOutputLayout::Fixed { fields } => fields,
            _ => return None,
        };
        let idx = fields.iter().position(|f| f.name == name)?;
        let total_output_bytes = fields.len() * FIELD_BYTE_LEN;
        if public_signals.len() < total_output_bytes {
            return None;
        }
        let output_start = public_signals.len() - total_output_bytes;
        let offset = output_start + idx * FIELD_BYTE_LEN;
        Some(&public_signals[offset..offset + FIELD_BYTE_LEN])
    }

    /// Extract all output fields from raw `public_signals` bytes.
    ///
    /// Returns a vec of `(name, &[u8])` pairs in field order.
    pub fn extract_all<'a>(
        &self,
        public_signals: &'a [u8],
    ) -> Option<Vec<(&'static str, &'a [u8])>> {
        let fields = match self {
            CircuitOutputLayout::Fixed { fields } => fields,
            CircuitOutputLayout::None => return Some(Vec::new()),
            CircuitOutputLayout::Dynamic => return None,
        };
        let total_output_bytes = fields.len() * FIELD_BYTE_LEN;
        if public_signals.len() < total_output_bytes {
            return None;
        }
        let output_start = public_signals.len() - total_output_bytes;
        Some(
            fields
                .iter()
                .enumerate()
                .map(|(i, f)| {
                    let offset = output_start + i * FIELD_BYTE_LEN;
                    (f.name, &public_signals[offset..offset + FIELD_BYTE_LEN])
                })
                .collect(),
        )
    }
}

// ── Public input layout (fields at the HEAD of public_signals) ──────────────

/// Describes the public input fields of a circuit.
/// Inputs sit at the **start** of `public_signals`, before any return values.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CircuitInputLayout {
    /// Fixed number of named `Field`-sized inputs, known at compile time.
    Fixed { fields: &'static [OutputField] },
    /// No public inputs.
    None,
}

impl CircuitInputLayout {
    /// Extract a named public input field from raw `public_signals` bytes.
    /// Inputs sit at the **start** of `public_signals`.
    pub fn extract_field<'a>(&self, public_signals: &'a [u8], name: &str) -> Option<&'a [u8]> {
        let fields = match self {
            CircuitInputLayout::Fixed { fields } => fields,
            _ => return None,
        };
        let idx = fields.iter().position(|f| f.name == name)?;
        let offset = idx * FIELD_BYTE_LEN;
        let end = offset + FIELD_BYTE_LEN;
        if public_signals.len() < end {
            return None;
        }
        Some(&public_signals[offset..end])
    }
}

/// C3 — Share encryption public inputs.
pub const SHARE_ENCRYPTION_INPUTS: &[OutputField] = &[
    f("expected_pk_commitment"),
    f("expected_message_commitment"),
];

/// C6 — Threshold share decryption public inputs.
pub const THRESHOLD_SHARE_DECRYPTION_INPUTS: &[OutputField] =
    &[f("expected_sk_commitment"), f("expected_e_sm_commitment")];

// ── Per-circuit output field constants ──────────────────────────────────────

const fn f(name: &'static str) -> OutputField {
    OutputField { name }
}

/// C0 — BFV public key proof.
pub const PK_BFV_OUTPUTS: &[OutputField] = &[f("pk_commitment")];

/// C1 — Threshold public key generation.
pub const PK_GENERATION_OUTPUTS: &[OutputField] =
    &[f("sk_commitment"), f("pk_commitment"), f("e_sm_commitment")];

/// C2d — Share computation chunk batch.
pub const SHARE_COMPUTATION_CHUNK_BATCH_OUTPUTS: &[OutputField] = &[f("commitment")];

/// C2 — Share computation (final wrapper).
pub const SHARE_COMPUTATION_OUTPUTS: &[OutputField] = &[f("key_hash"), f("commitment")];

/// C4 — DKG share decryption.
pub const DKG_SHARE_DECRYPTION_OUTPUTS: &[OutputField] = &[f("commitment")];

/// C5 — Public key aggregation.
pub const PK_AGGREGATION_OUTPUTS: &[OutputField] = &[f("commitment")];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_single_output_field() {
        let layout = CircuitOutputLayout::Fixed {
            fields: PK_BFV_OUTPUTS,
        };
        // 32 bytes pub input + 32 bytes output
        let mut signals = vec![0xAAu8; 64];
        signals[32..].copy_from_slice(&[0xBB; 32]);
        let commitment = layout.extract_field(&signals, "pk_commitment").unwrap();
        assert_eq!(commitment, &[0xBB; 32]);
    }

    #[test]
    fn extract_c1_pk_commitment_from_middle() {
        let layout = CircuitOutputLayout::Fixed {
            fields: PK_GENERATION_OUTPUTS,
        };
        // C1 has no pub inputs, only 3 outputs = 96 bytes total
        let mut signals = vec![0u8; 96];
        signals[0..32].copy_from_slice(&[0x11; 32]); // sk_commitment
        signals[32..64].copy_from_slice(&[0x22; 32]); // pk_commitment
        signals[64..96].copy_from_slice(&[0x33; 32]); // e_sm_commitment

        assert_eq!(
            layout.extract_field(&signals, "sk_commitment").unwrap(),
            &[0x11; 32]
        );
        assert_eq!(
            layout.extract_field(&signals, "pk_commitment").unwrap(),
            &[0x22; 32]
        );
        assert_eq!(
            layout.extract_field(&signals, "e_sm_commitment").unwrap(),
            &[0x33; 32]
        );
    }

    #[test]
    fn extract_c5_output_after_pub_inputs() {
        let layout = CircuitOutputLayout::Fixed {
            fields: PK_AGGREGATION_OUTPUTS,
        };
        // C5 has H pub input fields + 1 output. Simulate H=3 → 128 bytes total.
        let mut signals = vec![0xAA; 128]; // 3 * 32 pub inputs
        signals[96..128].copy_from_slice(&[0xFF; 32]); // 1 output at the end
        let commitment = layout.extract_field(&signals, "commitment").unwrap();
        assert_eq!(commitment, &[0xFF; 32]);
    }

    #[test]
    fn extract_c2_two_outputs() {
        let layout = CircuitOutputLayout::Fixed {
            fields: SHARE_COMPUTATION_OUTPUTS,
        };
        // C2 has 1 pub input (key_hash) + 2 outputs = 96 bytes
        let mut signals = vec![0x00; 96];
        signals[32..64].copy_from_slice(&[0xAA; 32]); // key_hash output
        signals[64..96].copy_from_slice(&[0xBB; 32]); // commitment output

        assert_eq!(
            layout.extract_field(&signals, "key_hash").unwrap(),
            &[0xAA; 32]
        );
        assert_eq!(
            layout.extract_field(&signals, "commitment").unwrap(),
            &[0xBB; 32]
        );
    }

    #[test]
    fn extract_nonexistent_field_returns_none() {
        let layout = CircuitOutputLayout::Fixed {
            fields: PK_BFV_OUTPUTS,
        };
        let signals = vec![0u8; 32];
        assert!(layout.extract_field(&signals, "nonexistent").is_none());
    }

    #[test]
    fn extract_from_void_circuit_returns_none() {
        let layout = CircuitOutputLayout::None;
        let signals = vec![0u8; 64];
        assert!(layout.extract_field(&signals, "anything").is_none());
    }

    #[test]
    fn extract_from_dynamic_circuit_returns_none() {
        let layout = CircuitOutputLayout::Dynamic;
        let signals = vec![0u8; 256];
        assert!(layout.extract_field(&signals, "anything").is_none());
    }

    #[test]
    fn signals_too_short_returns_none() {
        let layout = CircuitOutputLayout::Fixed {
            fields: PK_GENERATION_OUTPUTS,
        };
        // Need 96 bytes for 3 outputs, only 64 available
        let signals = vec![0u8; 64];
        assert!(layout.extract_field(&signals, "pk_commitment").is_none());
    }

    #[test]
    fn extract_all_c1_outputs() {
        let layout = CircuitOutputLayout::Fixed {
            fields: PK_GENERATION_OUTPUTS,
        };
        let mut signals = vec![0u8; 96];
        signals[0..32].copy_from_slice(&[0x11; 32]);
        signals[32..64].copy_from_slice(&[0x22; 32]);
        signals[64..96].copy_from_slice(&[0x33; 32]);

        let all = layout.extract_all(&signals).unwrap();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].0, "sk_commitment");
        assert_eq!(all[1].0, "pk_commitment");
        assert_eq!(all[2].0, "e_sm_commitment");
        assert_eq!(all[1].1, &[0x22; 32]);
    }

    #[test]
    fn field_count() {
        assert_eq!(
            CircuitOutputLayout::Fixed {
                fields: PK_GENERATION_OUTPUTS
            }
            .field_count(),
            Some(3)
        );
        assert_eq!(CircuitOutputLayout::None.field_count(), Some(0));
        assert_eq!(CircuitOutputLayout::Dynamic.field_count(), None);
    }
}
