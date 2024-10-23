use crate::{decrypt_data, encrypt_data};
use actix::prelude::*;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use data::{Checkpoint, FromSnapshotWithParams, Repository, Snapshot};
use enclave_core::{
    BusError, CiphernodeSelected, CiphertextOutputPublished, DecryptionshareCreated, Die,
    EnclaveErrorType, EnclaveEvent, EventBus, FromError, KeyshareCreated,
};
use fhe::{DecryptCiphertext, Fhe};
use serde::{Deserialize, Serialize};
use std::env;
use std::{process, sync::Arc};
use tracing::warn;
use zeroize::Zeroizing;

pub struct Keyshare {
    fhe: Arc<Fhe>,
    store: Repository<KeyshareState>,
    bus: Addr<EventBus>,
    secret: Option<Vec<u8>>,
    address: String,
}

impl Actor for Keyshare {
    type Context = actix::Context<Self>;
}

pub struct KeyshareParams {
    pub bus: Addr<EventBus>,
    pub store: Repository<KeyshareState>,
    pub fhe: Arc<Fhe>,
    pub address: String,
}

#[derive(Serialize, Deserialize)]
pub struct KeyshareState {
    secret: Option<Vec<u8>>,
}

impl Keyshare {
    pub fn new(params: KeyshareParams) -> Self {
        Self {
            bus: params.bus,
            fhe: params.fhe,
            store: params.store,
            secret: None,
            address: params.address,
        }
    }

    fn set_secret(&mut self, mut data: Vec<u8>) -> Result<()> {
        let encrypted = encrypt_data(&mut data)?;
        self.secret = Some(encrypted);
        Ok(())
    }

    fn get_secret(&self) -> Result<Vec<u8>> {
        let encrypted = self
            .secret
            .clone()
            .ok_or(anyhow!("No secret share available on Keyshare"))?;
        let decrypted = decrypt_data(&encrypted)?;
        Ok(decrypted)
    }
}

impl Snapshot for Keyshare {
    type Snapshot = KeyshareState;

    fn snapshot(&self) -> Self::Snapshot {
        KeyshareState {
            secret: self.secret.clone(),
        }
    }
}

impl Checkpoint for Keyshare {
    fn repository(&self) -> Repository<KeyshareState> {
        self.store.clone()
    }
}

#[async_trait]
impl FromSnapshotWithParams for Keyshare {
    type Params = KeyshareParams;
    async fn from_snapshot(params: Self::Params, snapshot: Self::Snapshot) -> Result<Self> {
        Ok(Self {
            bus: params.bus,
            fhe: params.fhe,
            store: params.store,
            secret: snapshot.secret,
            address: params.address,
        })
    }
}

impl Handler<EnclaveEvent> for Keyshare {
    type Result = ();

    fn handle(&mut self, event: EnclaveEvent, ctx: &mut actix::Context<Self>) -> Self::Result {
        match event {
            EnclaveEvent::CiphernodeSelected { data, .. } => ctx.notify(data),
            EnclaveEvent::CiphertextOutputPublished { data, .. } => ctx.notify(data),
            EnclaveEvent::E3RequestComplete { .. } => ctx.notify(Die),
            EnclaveEvent::Shutdown { .. } => ctx.notify(Die),
            _ => (),
        }
    }
}

impl Handler<CiphernodeSelected> for Keyshare {
    type Result = ();

    fn handle(&mut self, event: CiphernodeSelected, _: &mut actix::Context<Self>) -> Self::Result {
        let CiphernodeSelected { e3_id, .. } = event;

        // generate keyshare
        let Ok((secret, pubkey)) = self.fhe.generate_keyshare() else {
            self.bus.do_send(EnclaveEvent::from_error(
                EnclaveErrorType::KeyGeneration,
                anyhow!("Error creating Keyshare for {e3_id}"),
            ));
            return;
        };

        // Save secret on state
        let Ok(()) = self.set_secret(secret) else {
            self.bus.do_send(EnclaveEvent::from_error(
                EnclaveErrorType::KeyGeneration,
                anyhow!("Error encrypting Keyshare for {e3_id}"),
            ));
            return;
        };

        // Broadcast the KeyshareCreated message
        self.bus.do_send(EnclaveEvent::from(KeyshareCreated {
            pubkey,
            e3_id,
            node: self.address.clone(),
        }));

        // Write the snapshot to the store
        self.checkpoint()
    }
}

impl Handler<CiphertextOutputPublished> for Keyshare {
    type Result = ();

    fn handle(
        &mut self,
        event: CiphertextOutputPublished,
        _: &mut actix::Context<Self>,
    ) -> Self::Result {
        let CiphertextOutputPublished {
            e3_id,
            ciphertext_output,
        } = event;

        let Ok(secret) = self.get_secret() else {
            self.bus.err(
                EnclaveErrorType::Decryption,
                anyhow!("Secret not available for Keyshare for e3_id {e3_id}"),
            );
            return;
        };

        let Ok(decryption_share) = self.fhe.decrypt_ciphertext(DecryptCiphertext {
            ciphertext: ciphertext_output.clone(),
            unsafe_secret: secret,
        }) else {
            self.bus.err(
                EnclaveErrorType::Decryption,
                anyhow!("error decrypting ciphertext: {:?}", ciphertext_output),
            );
            return;
        };

        self.bus.do_send(EnclaveEvent::from(DecryptionshareCreated {
            e3_id,
            decryption_share,
            node: self.address.clone(),
        }));
    }
}

impl Handler<Die> for Keyshare {
    type Result = ();
    fn handle(&mut self, _: Die, ctx: &mut Self::Context) -> Self::Result {
        warn!("Keyshare is shutting down");
        ctx.stop()
    }
}
