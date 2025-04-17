use crate::{Fhe, FheRepositoryFactory, SharedRng};
use actix::Addr;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use data::{FromSnapshotWithParams, RepositoriesFactory, Snapshot};
use e3_request::{E3Context, E3ContextSnapshot, E3Extension, TypedKey};
use events::{BusError, E3Requested, EnclaveErrorType, EnclaveEvent, EventBus};
use hex;
use std::sync::Arc;

pub const FHE_KEY: TypedKey<Arc<Fhe>> = TypedKey::new("fhe");

/// TODO: move these to each package with access on MyStruct::launcher()
pub struct FheExtension {
    rng: SharedRng,
    bus: Addr<EventBus<EnclaveEvent>>,
}

impl FheExtension {
    pub fn create(bus: &Addr<EventBus<EnclaveEvent>>, rng: &SharedRng) -> Box<Self> {
        Box::new(Self {
            rng: rng.clone(),
            bus: bus.clone(),
        })
    }
}

const ERROR_FHE_FAILED_TO_DECODE: &str = "Failed to decode encoded FHE params";

#[async_trait]
impl E3Extension for FheExtension {
    fn on_event(&self, ctx: &mut E3Context, evt: &EnclaveEvent) {
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

        let params_bytes: Vec<u8> = params.iter().map(|&x| x as u8).collect();

        let Ok(fhe_inner) = Fhe::from_encoded(&params_bytes, seed, self.rng.clone()) else {
            self.bus.err(
                EnclaveErrorType::KeyGeneration,
                anyhow!(ERROR_FHE_FAILED_TO_DECODE),
            );
            return;
        };

        let fhe = Arc::new(fhe_inner);

        // FHE doesn't implement Checkpoint so we are going to store it manually
        let Ok(snapshot) = fhe.snapshot() else {
            self.bus.err(
                EnclaveErrorType::KeyGeneration,
                anyhow!("Failed to get snapshot"),
            );
            return;
        };
        ctx.repositories().fhe(&e3_id).write(&snapshot);
        let _ = ctx.set_dependency(FHE_KEY, fhe);
    }

    async fn hydrate(&self, ctx: &mut E3Context, snapshot: &E3ContextSnapshot) -> Result<()> {
        // No ID on the snapshot -> bail without reporting
        if !snapshot.contains("fhe") {
            return Ok(());
        };

        // No Snapshot returned from the store -> bail without reporting
        let Some(snap) = ctx.repositories().fhe(&ctx.e3_id).read().await? else {
            return Ok(());
        };

        let value = Arc::new(Fhe::from_snapshot(self.rng.clone(), snap).await?);
        ctx.set_dependency(FHE_KEY, value);

        Ok(())
    }
}
