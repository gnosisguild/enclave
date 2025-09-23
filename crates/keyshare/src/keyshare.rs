// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::prelude::*;
use anyhow::{anyhow, Result};
use e3_crypto::Cipher;
use e3_data::Persistable;
use e3_events::{
    BusError, CiphernodeSelected, CiphertextOutputPublished, DecryptionshareCreated, Die,
    E3RequestComplete, EnclaveErrorType, EnclaveEvent, EventBus, FromError, KeyshareCreated,
};
use e3_fhe::{DecryptCiphertext, Fhe};
use e3_utils::utility_types::ArcBytes;
use std::sync::Arc;
use tracing::warn;

pub struct Keyshare {
    fhe: Arc<Fhe>,
    bus: Addr<EventBus<EnclaveEvent>>,
    secret: Persistable<Vec<u8>>,
    address: String,
    cipher: Arc<Cipher>,
}

impl Actor for Keyshare {
    type Context = actix::Context<Self>;
}

pub struct KeyshareParams {
    pub bus: Addr<EventBus<EnclaveEvent>>,
    pub secret: Persistable<Vec<u8>>,
    pub fhe: Arc<Fhe>,
    pub address: String,
    pub cipher: Arc<Cipher>,
}

impl Keyshare {
    pub fn new(params: KeyshareParams) -> Self {
        Self {
            bus: params.bus,
            fhe: params.fhe,
            secret: params.secret,
            address: params.address,
            cipher: params.cipher,
        }
    }

    fn set_secret(&mut self, mut data: Vec<u8>) -> Result<()> {
        let encrypted = self.cipher.encrypt_data(&mut data)?;

        self.secret.set(encrypted);

        Ok(())
    }

    fn get_secret(&self) -> Result<Vec<u8>> {
        let encrypted = self
            .secret
            .get()
            .ok_or(anyhow!("State was not stored on keyshare"))?;

        let decrypted = self.cipher.decrypt_data(&encrypted)?;

        Ok(decrypted)
    }

    fn clear_secret(&mut self) {
        self.secret.clear();
    }
}

impl Handler<EnclaveEvent> for Keyshare {
    type Result = ();

    fn handle(&mut self, event: EnclaveEvent, ctx: &mut actix::Context<Self>) -> Self::Result {
        match event {
            EnclaveEvent::CiphernodeSelected { data, .. } => ctx.notify(data),
            EnclaveEvent::CiphertextOutputPublished { data, .. } => ctx.notify(data),
            EnclaveEvent::E3RequestComplete { data, .. } => ctx.notify(data),
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
        if let Err(err) = self.set_secret(secret) {
            self.bus.do_send(EnclaveEvent::from_error(
                EnclaveErrorType::KeyGeneration,
                err,
            ))
        };

        // Broadcast the KeyshareCreated message
        self.bus.do_send(EnclaveEvent::from(KeyshareCreated {
            pubkey,
            e3_id,
            node: self.address.clone(),
        }));
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

        let Some(ciphertext) = ciphertext_output.first() else {
            self.bus.err(
                EnclaveErrorType::Decryption,
                anyhow!("Ciphernode output array is empty!"),
            );
            return;
        };

        let Ok(decryption_share) = self.fhe.decrypt_ciphertext(DecryptCiphertext {
            ciphertext: ciphertext.extract_bytes(),
            unsafe_secret: secret,
        }) else {
            self.bus.err(
                EnclaveErrorType::Decryption,
                anyhow!("error decrypting ciphertext: {:?}", ciphertext_output),
            );
            return;
        };

        self.bus.do_send(EnclaveEvent::from(DecryptionshareCreated {
            party_id: 0, // Not used
            e3_id,
            decryption_share: vec![ArcBytes::from_bytes(decryption_share)],
            node: self.address.clone(),
        }));
    }
}

impl Handler<E3RequestComplete> for Keyshare {
    type Result = ();
    fn handle(&mut self, _: E3RequestComplete, ctx: &mut Self::Context) -> Self::Result {
        self.clear_secret();
        ctx.notify(Die);
    }
}

impl Handler<Die> for Keyshare {
    type Result = ();
    fn handle(&mut self, _: Die, ctx: &mut Self::Context) -> Self::Result {
        warn!("Keyshare is shutting down now");
        ctx.stop()
    }
}
