// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
use std::collections::HashMap;

pub fn to_ordered_vec<K, T>(source: HashMap<K, T>) -> Vec<T>
where
    K: Ord + Copy,
{
    // extract a vector
    let mut pairs: Vec<_> = source.into_iter().collect();

    // Ensure keys are sorted
    pairs.sort_by_key(|&(key, _)| key);

    // Extract to Vec of ThresholdShares in order
    pairs.into_iter().map(|(_, value)| value).collect()
}
