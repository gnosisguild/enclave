// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

/// This trait allows our keys to be responsive to multiple inputs
pub trait IntoKey {
    fn into_key(self) -> Vec<u8>;
}

/// Keys can be vectors of String
impl IntoKey for Vec<u8> {
    fn into_key(self) -> Vec<u8> {
        self
    }
}

/// Keys can be references to vectors of bytes (&Vec<u8>)
impl IntoKey for &Vec<u8> {
    fn into_key(self) -> Vec<u8> {
        self.clone()
    }
}

/// Keys can be vectors of String
impl IntoKey for Vec<String> {
    fn into_key(self) -> Vec<u8> {
        self.join("/").into_bytes()
    }
}

/// Keys can be vectors of &str
impl<'a> IntoKey for Vec<&'a str> {
    fn into_key(self) -> Vec<u8> {
        self.join("/").into_bytes()
    }
}

/// Keys can be String
impl IntoKey for String {
    fn into_key(self) -> Vec<u8> {
        self.into_bytes()
    }
}

/// Keys can be &String
impl IntoKey for &String {
    fn into_key(self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
}

/// Keys can be &str
impl<'a> IntoKey for &'a str {
    /// Produce a UTF-8 byte sequence from a borrowed string.
    ///
    /// # Examples
    ///
    /// ```
    /// let s = String::from("hello");
    /// let key = (&s).into_key();
    /// assert_eq!(key, b"hello".to_vec());
    /// ```
    ///
    /// # Returns
    ///
    /// A byte vector containing the UTF-8 encoding of the string.
    fn into_key(self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
}

/// Keys can be u128
impl IntoKey for u128 {
    /// Converts the integer into a big-endian byte vector suitable for use as a key.
    ///
    /// The returned bytes are in big-endian order to preserve numeric ordering when compared lexicographically.
    ///
    /// # Examples
    ///
    /// ```
    /// let key = 42u128.into_key();
    /// assert_eq!(key, 42u128.to_be_bytes().to_vec());
    /// ```
    fn into_key(self) -> Vec<u8> {
        // Ensuring big endian for ordering
        self.to_be_bytes().to_vec()
    }
}