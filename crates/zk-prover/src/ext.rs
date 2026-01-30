// SPDX-License-Identifier: LGPL-3.0-only

use crate::backend::ZkBackend;
use crate::prover::ZkProver;
use anyhow::Result;
use async_trait::async_trait;
use e3_events::EnclaveEvent;
use e3_request::{E3Context, E3ContextSnapshot, E3Extension, TypedKey};
use std::sync::Arc;
use tracing::info;

pub const ZK_PROVER_KEY: TypedKey<Arc<ZkProver>> = TypedKey::new("zk_prover");

pub struct ZkProofExtension {
    prover: Arc<ZkProver>,
}

impl ZkProofExtension {
    pub fn create(backend: &ZkBackend) -> Box<Self> {
        let prover = Arc::new(ZkProver::new(backend));
        Box::new(Self { prover })
    }

    pub fn with_prover(prover: Arc<ZkProver>) -> Box<Self> {
        Box::new(Self { prover })
    }
}

#[async_trait]
impl E3Extension for ZkProofExtension {
    fn on_event(&self, ctx: &mut E3Context, _evt: &EnclaveEvent) {
        if ctx.get_dependency(ZK_PROVER_KEY).is_some() {
            return;
        }

        info!("setting up ZkProver for e3_id={}", ctx.e3_id);
        ctx.set_dependency(ZK_PROVER_KEY, self.prover.clone());
    }

    async fn hydrate(&self, ctx: &mut E3Context, snapshot: &E3ContextSnapshot) -> Result<()> {
        if !snapshot.contains("zk_prover") {
            return Ok(());
        }

        ctx.set_dependency(ZK_PROVER_KEY, self.prover.clone());
        Ok(())
    }
}
