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

/// truncate a string
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

pub fn colorize<T: std::fmt::Display>(s: T, color: Color) -> String {
    format!("\x1b[{}m{}\x1b[0m", color as u8, s)
}
