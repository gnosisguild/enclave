// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::{fmt::Debug, fs, io::Write, path::PathBuf};

use tracing::{error, trace};

pub fn write_file_with_dirs(path: &PathBuf, content: &[u8]) -> std::io::Result<()> {
    let abs_path = if path.is_absolute() {
        path.clone()
    } else {
        let cwd = std::env::current_dir()?;
        cwd.join(path)
    };

    match abs_path.to_str() {
        Some(s) => trace!(path = s, "Writing to path"),
        None => error!(path=?abs_path, "Cannot parse path"),
    };

    // Ensure the directory structure exists
    if let Some(parent) = abs_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Open the file (creates it if it doesn't exist) and write the content
    let mut file = fs::File::create(&abs_path)?;
    file.write_all(content)?;
    trace!(
        path = abs_path.to_str().unwrap(),
        "File written successfully!"
    );
    Ok(())
}

fn strip_outer_quotes(s: &str) -> &str {
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

fn truncate(s: &str) -> String {
    let limit = 300;
    let cutoff = limit - (limit / 3);
    if s.len() <= limit {
        s.to_string()
    } else {
        let start = &s[..cutoff];
        let end = &s[s.len() - (limit - cutoff)..];
        format!("{}...{}", start, end)
    }
}

pub fn d<T: Debug>(value: T) -> String {
    let debug_str = format!("{:?}", value);
    let unquoted = strip_outer_quotes(&debug_str);
    truncate(unquoted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_long_string_truncation() {
        let long_string = "a".repeat(500);
        let result = d(long_string);

        assert_eq!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa...aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", result);
    }
}
