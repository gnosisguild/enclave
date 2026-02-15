// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

// NOTE: Currently this is here as everything depends on utils. We could consider moving this
// closer to the configuration if we need to make this dynamic or create a create just for this.

// Max message
pub const MAILBOX_LIMIT: usize = 256;
pub const MAILBOX_LIMIT_LARGE: usize = 256 * 10;
