
use router::{E3Feature, E3RequestContext, E3RequestContextSnapshot, RepositoriesFactory};
use actix::Addr;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use data::{FromSnapshotWithParams, Snapshot};
use enclave_core::{BusError, E3Requested, EnclaveErrorType, EnclaveEvent, EventBus};
use fhe::{Fhe, SharedRng};
use std::sync::Arc;

pub struct FheFeature {
    rng: SharedRng,
    bus: Addr<EventBus>,
}

impl FheFeature {
    pub fn create(bus: &Addr<EventBus>, rng: &SharedRng) -> Box<Self> {
        Box::new(Self {
            rng: rng.clone(),
            bus: bus.clone(),
        })
    }
}

const ERROR_FHE_FAILED_TO_DECODE: &str = "Failed to decode encoded FHE params";

#[async_trait]
impl E3Feature for FheFeature {
    fn on_event(&self, ctx: &mut crate::E3RequestContext, evt: &EnclaveEvent) {
        // Saving the fhe on Committee Requested
        let EnclaveEvent::E3Requested { data, .. } = evt else {
            return;
        };

        let E3Requested {
            params,
            seed,
            e3_id,
            ..
        } = data.clone();

        let Ok(fhe_inner) = Fhe::from_encoded(&params, seed, self.rng.clone()) else {
            self.bus.err(
                EnclaveErrorType::KeyGeneration,
                anyhow!(ERROR_FHE_FAILED_TO_DECODE),
            );
            return;
        };

        let fhe = Arc::new(fhe_inner);

        // FHE doesn't implement Checkpoint so we are going to store it manually
        ctx.repositories().fhe(&e3_id).write(&fhe.snapshot());

        let _ = ctx.set_fhe(fhe);
    }

    async fn hydrate(
        &self,
        ctx: &mut E3RequestContext,
        snapshot: &E3RequestContextSnapshot,
    ) -> Result<()> {
        // No ID on the snapshot -> bail without reporting
        if !snapshot.fhe {
            return Ok(());
        };

        // No Snapshot returned from the store -> bail without reporting
        let Some(snap) = ctx.repositories().fhe(&ctx.e3_id).read().await? else {
            return Ok(());
        };

        let value = Arc::new(Fhe::from_snapshot(self.rng.clone(), snap).await?);
        ctx.set_fhe(value);

        Ok(())
    }
}
