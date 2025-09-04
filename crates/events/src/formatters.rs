// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use core::fmt;

// Custom formatter function for hex display
pub fn hexf(data: &[u8], f: &mut fmt::Formatter) -> fmt::Result {
    write!(
        f,
        "{}",
        truncate(
            data.iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>()
        )
    )
}

pub fn hexf_bytes_slice<T: AsRef<[u8]>>(data: &[T], f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "[")?;
    for (i, bytes) in data.iter().enumerate() {
        if i > 0 {
            write!(f, ", ")?;
        }
        hexf(bytes.as_ref(), f)?;
    }
    write!(f, "]")
}

pub fn hexf_3d_bytes(data: &Vec<Vec<Vec<u8>>>, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "[")?;
    for (i, outer) in data.iter().enumerate() {
        if i > 0 {
            write!(f, ", ")?;
        }
        write!(f, "[")?;
        for (j, inner) in outer.iter().enumerate() {
            if j > 0 {
                write!(f, ", ")?;
            }
            hexf(inner, f)?;
        }
        write!(f, "]")?;
    }
    write!(f, "]")
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
