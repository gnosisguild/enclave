// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use core::fmt;

// Custom formatter function for hex display
pub fn hexf(data: &[u8], f: &mut fmt::Formatter) -> fmt::Result {
    let bytes: &[u8] = data.as_ref();

    write!(
        f,
        "{}",
        truncate(
            bytes
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>()
        )
    )
}

/// Truncates a hex-like string for compact display, prefixing short values with `0x` and
/// summarizing long values with their length and leading/trailing segments.
///
/// For input strings with length <= 100, the function returns the input prefixed with `0x`.
/// For longer inputs it returns `"<bytes(len):0x<start>..<end>>"`, where `start` is the
/// first 25 characters and `end` is the last 25 characters of the original string.
///
/// # Examples
///
/// ```
/// // Short input is prefixed with 0x
/// let short = "deadbeef".to_string();
/// assert_eq!(crate::truncate(short), "0xdeadbeef");
///
/// // Long input is summarized with length and leading/trailing segments
/// let long = String::from("a").repeat(120);
/// let expected = format!("<bytes({}):0x{}..{}>", 120, "a".repeat(25), "a".repeat(25));
/// assert_eq!(crate::truncate(long), expected);
/// ```
pub fn truncate(s: String) -> String {
    let threshold = 100; // will leave it
    let limit = 50;
    let cutoff = limit / 2;
    if s.len() <= threshold {
        format!("0x{}", s)
    } else {
        let start = &s[..cutoff];
        let end = &s[s.len() - (limit - cutoff)..];
        format!("<bytes({}):0x{}..{}>", s.len(), start, end)
    }
}

pub enum Color {
    Black = 30,
    Red = 31,
    Green = 32,
    Yellow = 33,
    Blue = 34,
    Magenta = 35,
    Cyan = 36,
    White = 37,
    BrightBlack = 90,
    BrightRed = 91,
    BrightGreen = 92,
    BrightYellow = 93,
    BrightBlue = 94,
    BrightMagenta = 95,
    BrightCyan = 96,
    BrightWhite = 97,
}

/// Wraps a Displayable value with ANSI escape codes to apply the specified color.
///
/// The returned string contains an ANSI sequence that sets the given color, the
/// formatted value, and a reset sequence to restore terminal formatting.
///
/// # Examples
///
/// ```
/// let s = colorize("hello", Color::Red);
/// assert!(s.starts_with("\x1b[31m"));
/// assert!(s.ends_with("\x1b[0m"));
/// ```
pub fn colorize<T: std::fmt::Display>(s: T, color: Color) -> String {
    format!("\x1b[{}m{}\x1b[0m", color as u8, s)
}