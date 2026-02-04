// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use eyre::{Context, Result};

/// Merges two JSON objects into one. All keys end up at the same level; keys from `b` overwrite `a` on conflict.
pub fn merge_json_objects(a: serde_json::Value, b: serde_json::Value) -> Result<String> {
    let mut merged: serde_json::Map<String, serde_json::Value> =
        serde_json::from_value(a).with_context(|| "First value is not a JSON object")?;
    let other: serde_json::Map<String, serde_json::Value> =
        serde_json::from_value(b).with_context(|| "Second value is not a JSON object")?;
    merged.extend(other);
    serde_json::to_string(&merged).with_context(|| "Failed to serialize merged JSON")
}
