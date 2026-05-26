// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! E3 fault attribution and accusation quorum protocol.
//!
//! Extracted from `e3-zk-prover` — accusation management is protocol-level
//! fault-handling, not ZK infrastructure.

pub mod accusation_manager;
pub mod accusation_manager_ext;
pub mod commitment_consistency_checker;
pub mod commitment_consistency_checker_ext;

pub use accusation_manager::AccusationManager;
pub use accusation_manager_ext::AccusationManagerExtension;
pub use commitment_consistency_checker::CommitmentConsistencyChecker;
pub use commitment_consistency_checker_ext::CommitmentConsistencyCheckerExtension;
