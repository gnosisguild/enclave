// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
use std::sync::Arc;

/// Reference count bytes so event can be cloned and shared between threads
pub type Bytes = Arc<Vec<u8>>;
