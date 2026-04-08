// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

/// A trait for types that can be serialized to and deserialized from raw bytes.
///
/// This trait is used by the [`BytesSerde`] derive macro to implement `serde::Serialize`
/// and `serde::Deserialize`. Human-readable formats (e.g. JSON) encode the bytes as
/// a `0x`-prefixed hex string, while binary formats (e.g. bincode) use raw bytes directly.
///
/// # Implementors
///
/// Implementations should ensure that `try_from_bytes(self.as_bytes().to_vec())`
/// round-trips successfully.
///
/// # Example
///
/// ```rust
/// use e3_utils::AsBytesSerde;
///
/// pub struct EventId(pub [u8; 32]);
///
/// impl AsBytesSerde for EventId {
///     fn as_bytes(&self) -> &[u8] {
///         &self.0
///     }
///
///     fn try_from_bytes(bytes: Vec<u8>) -> Result<Self, String> {
///         let arr: [u8; 32] = bytes
///             .try_into()
///             .map_err(|_| "EventId requires exactly 32 bytes".to_string())?;
///         Ok(EventId(arr))
///     }
/// }
/// ```
pub trait AsBytesSerde: Sized {
    /// Returns the byte representation of this type.
    fn as_bytes(&self) -> &[u8];

    /// Attempts to construct an instance from a byte vector.
    ///
    /// # Errors
    ///
    /// Returns a descriptive error string if the bytes are invalid for this type
    /// (e.g. wrong length or malformed content).
    fn try_from_bytes(bytes: Vec<u8>) -> Result<Self, String>;
}
