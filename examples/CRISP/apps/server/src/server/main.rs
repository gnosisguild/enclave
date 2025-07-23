// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crisp::server;

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    server::start()
}
