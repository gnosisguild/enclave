use crate::{Keyshare, KeyshareParams, KeyshareRepositoryFactory};
use actix::{Actor, Addr};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use data::{AutoPersist, RepositoriesFactory};
use e3_crypto::Cipher;
use e3_request::{E3Context, E3ContextSnapshot, E3Extension};
use events::{BusError, EnclaveErrorType, EnclaveEvent, EventBus};
use fhe::ext::FHE_KEY;
use std::sync::Arc;

pub struct KeyshareExtension {
    bus: Addr<EventBus<EnclaveEvent>>,
    address: String,
    cipher: Arc<Cipher>,
}

impl KeyshareExtension {
    pub fn create(
        bus: &Addr<EventBus<EnclaveEvent>>,
        address: &str,
        cipher: &Arc<Cipher>,
    ) -> Box<Self> {
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
impl E3Extension for KeyshareExtension {
    fn on_event(&self, ctx: &mut E3Context, evt: &EnclaveEvent) {
        // if this is NOT a CiphernodeSelected event then ignore
        let EnclaveEvent::CiphernodeSelected { data, .. } = evt else {
            return;
        };

        // Has the FHE dependency been already setup? (hint: it should have)
        let Some(fhe) = ctx.get_dependency(FHE_KEY) else {
            self.bus.err(
                EnclaveErrorType::KeyGeneration,
                anyhow!(ERROR_KEYSHARE_FHE_MISSING),
            );
            return;
        };

        let e3_id = data.clone().e3_id;
        let repo = ctx.repositories().keyshare(&e3_id);
        let container = repo.send(None); // New container with None

        ctx.set_event_recipient(
            "keyshare",
            Some(
                Keyshare::new(KeyshareParams {
                    bus: self.bus.clone(),
                    secret: container,
                    fhe: fhe.clone(),
                    address: self.address.clone(),
                    cipher: self.cipher.clone(),
                })
                .start()
                .into(),
            ),
        );
    }

    async fn hydrate(&self, ctx: &mut E3Context, snapshot: &E3ContextSnapshot) -> Result<()> {
        // No keyshare on the snapshot -> bail
        if !snapshot.contains("keyshare") {
            return Ok(());
        };

        // Get the saved state as a persistable
        let sync_secret = ctx.repositories().keyshare(&snapshot.e3_id).load().await?;

        // No Snapshot returned from the sync_secret -> bail
        if !sync_secret.has() {
            return Ok(());
        };

        // Has the FHE dependency been already setup? (hint: it should have)
        let Some(fhe) = ctx.get_dependency(FHE_KEY) else {
            self.bus.err(
                EnclaveErrorType::KeyGeneration,
                anyhow!(ERROR_KEYSHARE_FHE_MISSING),
            );
            return Ok(());
        };

        // Construct from snapshot
        let value = Keyshare::new(KeyshareParams {
            fhe: fhe.clone(),
            bus: self.bus.clone(),
            secret: sync_secret,
            address: self.address.clone(),
            cipher: self.cipher.clone(),
        })
        .start()
        .into();

        // send to context
        ctx.set_event_recipient("keyshare", Some(value));

        Ok(())
    }
}
