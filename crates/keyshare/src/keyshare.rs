// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::prelude::*;
use anyhow::{anyhow, Context as AnyhowContext, Result};
use e3_crypto::Cipher;
use e3_data::Persistable;
use e3_events::{
    prelude::*, trap, BusHandle, CiphernodeSelected, CiphertextOutputPublished,
    DecryptionshareCreated, Die, E3RequestComplete, EType, EnclaveEvent, EnclaveEventData,
    KeyshareCreated,
};
use e3_fhe::{DecryptCiphertext, Fhe};
use e3_utils::utility_types::ArcBytes;
use std::sync::Arc;
use tracing::warn;

pub struct Keyshare {
    fhe: Arc<Fhe>,
    bus: BusHandle,
    secret: Persistable<Vec<u8>>,
    address: String,
    cipher: Arc<Cipher>,
}

impl Actor for Keyshare {
    type Context = actix::Context<Self>;
}

pub struct KeyshareParams {
    pub bus: BusHandle,
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
        match event.into_data() {
            EnclaveEventData::CiphernodeSelected(data) => ctx.notify(data),
            EnclaveEventData::CiphertextOutputPublished(data) => ctx.notify(data),
            EnclaveEventData::E3RequestComplete(data) => ctx.notify(data),
            EnclaveEventData::Shutdown(_) => ctx.notify(Die),
            _ => (),
        }
    }
}

impl Handler<CiphernodeSelected> for Keyshare {
    type Result = ();

    fn handle(&mut self, event: CiphernodeSelected, _: &mut actix::Context<Self>) -> Self::Result {
        trap(EType::KeyGeneration, &self.bus.clone(), || {
            let CiphernodeSelected { e3_id, .. } = event;

            // generate keyshare
            let (secret, pubkey) = self
                .fhe
                .generate_keyshare()
                .with_context(|| format!("Error creating Keyshare for {}", e3_id))?;

            // Save secret on state
            self.set_secret(secret)?;

            // Broadcast the KeyshareCreated message
            self.bus.publish(KeyshareCreated {
                pubkey,
                e3_id,
                node: self.address.clone(),
            })?;

            Ok(())
        })
    }
}

impl Handler<CiphertextOutputPublished> for Keyshare {
    type Result = ();

    fn handle(
        &mut self,
        event: CiphertextOutputPublished,
        _: &mut actix::Context<Self>,
    ) -> Self::Result {
        trap(EType::KeyGeneration, &self.bus.clone(), || {
            let CiphertextOutputPublished {
                e3_id,
                ciphertext_output,
            } = event;

            let secret = self.get_secret()?;

            let ciphertext = ciphertext_output
                .first()
                .ok_or(anyhow!("Ciphernode output array is empty!"))?;

            let decryption_share = self.fhe.decrypt_ciphertext(DecryptCiphertext {
                ciphertext: ciphertext.extract_bytes(),
                unsafe_secret: secret,
            })?;

            self.bus.publish(DecryptionshareCreated {
                party_id: 0, // Not used
                e3_id,
                decryption_share: vec![ArcBytes::from_bytes(&decryption_share)],
                node: self.address.clone(),
            })?;

            Ok(())
        })
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
