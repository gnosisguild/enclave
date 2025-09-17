// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use core::fmt;

// Custom formatter function for hex display
pub fn hexf(data: &[u8], f: &mut fmt::Formatter) -> fmt::Result {
    let bytes = data.as_ref();

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
fn truncate(s: String) -> String {
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
