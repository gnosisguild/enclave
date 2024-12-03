use crate::{Keyshare, KeyshareParams};
use actix::Addr;
use cipher::Cipher;
use data::{FromSnapshotWithParams, Snapshot};
use enclave_core::{EnclaveErrorType, EnclaveEvent, EventBus};
use router::{E3RequestContext, E3RequestContextSnapshot, RepositoriesFactory};
use std::sync::Arc;

pub struct KeyshareFeature {
    bus: Addr<EventBus>,
    address: String,
    cipher: Arc<Cipher>,
}

impl KeyshareFeature {
    pub fn create(bus: &Addr<EventBus>, address: &str, cipher: &Arc<Cipher>) -> Box<Self> {
        Box::new(Self {
            bus: bus.clone(),
            address: address.to_owned(),
            cipher: cipher.to_owned(),
        })
    }
}

const ERROR_KEYSHARE_FHE_MISSING: &str =
    "Could not create Keyshare because the fhe instance it depends on was not set on the context.";

#[async_trait]
impl router::E3Feature for KeyshareFeature {
    fn on_event(&self, ctx: &mut E3RequestContext, evt: &EnclaveEvent) {
        // Save Ciphernode on CiphernodeSelected
        let EnclaveEvent::CiphernodeSelected { data, .. } = evt else {
            return;
        };

        let Some(fhe) = ctx.get_fhe() else {
            self.bus.err(
                EnclaveErrorType::KeyGeneration,
                anyhow!(ERROR_KEYSHARE_FHE_MISSING),
            );
            return;
        };

        let e3_id = data.clone().e3_id;

        ctx.set_event_recipient(
            "keyshare",
            Some(
                Keyshare::new(KeyshareParams {
                    bus: self.bus.clone(),
                    store: ctx.repositories().keyshare(&e3_id),
                    fhe: fhe.clone(),
                    address: self.address.clone(),
                    cipher: self.cipher.clone(),
                })
                .start()
                .into(),
            ),
        );
    }

    async fn hydrate(
        &self,
        ctx: &mut E3RequestContext,
        snapshot: &E3RequestContextSnapshot,
    ) -> Result<()> {
        // No ID on the snapshot -> bail
        if !snapshot.contains("keyshare") {
            return Ok(());
        };

        let store = ctx.repositories().keyshare(&snapshot.e3_id);

        // No Snapshot returned from the store -> bail
        let Some(snap) = store.read().await? else {
            return Ok(());
        };

        // Get deps
        let Some(fhe) = ctx.fhe.clone() else {
            self.bus.err(
                EnclaveErrorType::KeyGeneration,
                anyhow!(ERROR_KEYSHARE_FHE_MISSING),
            );
            return Ok(());
        };

        // Construct from snapshot
        let value = Keyshare::from_snapshot(
            KeyshareParams {
                fhe,
                bus: self.bus.clone(),
                store,
                address: self.address.clone(),
                cipher: self.cipher.clone(),
            },
            snap,
        )
        .await?
        .start();

        // send to context
        ctx.set_event_recipient("keyshare", Some(value.into()));

        Ok(())
    }
}
