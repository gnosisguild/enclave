// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use core::fmt;
use std::sync::LazyLock;

use regex::Regex;

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

#[derive(Clone, Copy)]
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

// Pre-compile the regex for efficiency
static EVENT_ID_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"EventId\(0x([a-fA-F0-9]+)\)").unwrap());

/// Colorizes `EventId(0x...)` patterns in the debug output of a value.
fn hash_to_color(hex_str: &str) -> u8 {
    let hash: u32 = hex_str
        .bytes()
        .fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
    22 + (hash as u8 % 200) // 200 colors, skipping dark ones
}

pub fn colorize_event_ids<T: fmt::Debug>(value: &T) -> String {
    let s = format!("{:?}", value);
    let mut result = String::with_capacity(s.len() + 100);
    let mut last_end = 0;
    for cap in EVENT_ID_RE.captures_iter(&s) {
        let full_match = cap.get(0).unwrap();
        let hex_str = cap.get(1).unwrap().as_str();
        result.push_str(&s[last_end..full_match.start()]);
        let color = hash_to_color(hex_str);
        result.push_str(&format!(
            "\x1b[38;5;{}m{}\x1b[0m",
            color,
            full_match.as_str()
        ));
        last_end = full_match.end();
    }
    result.push_str(&s[last_end..]);
    result
}
