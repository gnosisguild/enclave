// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! IO actor for ZK proof generation and verification.
//! This actor handles actual disk-based operations for proving and verifying.
//!
//! This is an IO actor - it performs file system operations.

use actix::{Actor, Context, Handler};
use tracing::{debug, error};

use crate::{ZkBackend, ZkProver};

use super::proof_verification::{ZkVerificationRequest, ZkVerificationResponse};

/// IO actor that handles ZK proof generation and verification.
pub struct ZkActor {
    prover: ZkProver,
}

impl ZkActor {
    pub fn new(backend: &ZkBackend) -> Self {
        Self {
            prover: ZkProver::new(backend),
        }
    }
}

impl Actor for ZkActor {
    type Context = Context<Self>;
}

impl Handler<ZkVerificationRequest> for ZkActor {
    type Result = ();

    fn handle(&mut self, msg: ZkVerificationRequest, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Verifying proof for circuit: {}", msg.proof.circuit);

        let e3_id_str = msg.e3_id.to_string();
        let result = self.prover.verify_proof(
            msg.proof.circuit,
            &msg.proof.data,
            &msg.proof.public_signals,
            &e3_id_str,
        );

        let response = match result {
            Ok(true) => {
                debug!("Proof verification successful");
                ZkVerificationResponse {
                    verified: true,
                    error: None,
                    e3_id: msg.e3_id,
                    key: msg.key,
                }
            }
            Ok(false) => {
                error!("Proof verification failed");
                ZkVerificationResponse {
                    verified: false,
                    error: Some("Verification returned false".to_string()),
                    e3_id: msg.e3_id,
                    key: msg.key,
                }
            }
            Err(e) => {
                error!("Proof verification error: {}", e);
                ZkVerificationResponse {
                    verified: false,
                    error: Some(e.to_string()),
                    e3_id: msg.e3_id,
                    key: msg.key,
                }
            }
        };

        // Send response back to the sender
        msg.sender.do_send(response);
    }
}
