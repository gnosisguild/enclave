// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use e3_zk_helpers::utils::to_string_1d_vec;
use num_bigint::BigInt;
use serde_json;

pub fn map_witness_2d_vector_to_json(values: &Vec<Vec<BigInt>>) -> Vec<serde_json::Value> {
    values
        .iter()
        .map(|value| {
            serde_json::json!({
                "coefficients": to_string_1d_vec(value)
            })
        })
        .collect()
}
