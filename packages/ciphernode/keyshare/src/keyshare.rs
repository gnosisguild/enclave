use actix::prelude::*;
use anyhow::{anyhow, Context, Result};
use data::{DataStore, Get, Insert};
use enclave_core::{
    CiphernodeSelected, CiphertextOutputPublished, DecryptionshareCreated, Die, EnclaveErrorType,
    EnclaveEvent, EventBus, FromError, KeyshareCreated,
};
use fhe::{DecryptCiphertext, Fhe};
use std::sync::Arc;

pub struct Keyshare {
    fhe: Arc<Fhe>,
    data: DataStore,
    bus: Addr<EventBus>,
    address: String,
}

impl Actor for Keyshare {
    type Context = actix::Context<Self>;
}

impl Keyshare {
    pub fn new(bus: Addr<EventBus>, data: DataStore, fhe: Arc<Fhe>, address: &str) -> Self {
        Self {
            bus,
            fhe,
            data,
            address: address.to_string(),
        }
    }
}

impl Handler<EnclaveEvent> for Keyshare {
    type Result = ();

    fn handle(&mut self, event: EnclaveEvent, ctx: &mut actix::Context<Self>) -> Self::Result {
        match event {
            EnclaveEvent::CiphernodeSelected { data, .. } => ctx.notify(data),
            EnclaveEvent::CiphertextOutputPublished { data, .. } => ctx.notify(data),
            EnclaveEvent::E3RequestComplete { .. } => ctx.notify(Die),
            _ => (),
        }
    }
}

impl Handler<CiphernodeSelected> for Keyshare {
    type Result = ();

    fn handle(&mut self, event: CiphernodeSelected, _: &mut actix::Context<Self>) -> Self::Result {
        let CiphernodeSelected { e3_id, .. } = event;

        // generate keyshare
        let Ok((sk, pubkey)) = self.fhe.generate_keyshare() else {
            self.bus.do_send(EnclaveEvent::from_error(
                EnclaveErrorType::KeyGeneration,
                anyhow!("Error creating Keyshare"),
            ));
            return;
        };

        // TODO: decrypt from FHE actor
        // save encrypted key against e3_id/sk
        // reencrypt secretkey locally with env var - this is so we don't have to serialize a secret
        // best practice would be as you boot up a node you enter in a configured password from
        // which we derive a kdf which gets used to generate this key
        self.data
            .write(Insert(format!("{}/sk", e3_id).into(), sk));

        // save public key against e3_id/pk
        self.data
            .write(Insert(format!("{}/pk", e3_id).into(), pubkey.clone()));

        // broadcast the KeyshareCreated message
        let event = EnclaveEvent::from(KeyshareCreated {
            pubkey,
            e3_id,
            node: self.address.clone(),
        });
        self.bus.do_send(event);
    }
}

impl Handler<CiphertextOutputPublished> for Keyshare {
    type Result = ResponseFuture<()>;

    fn handle(
        &mut self,
        event: CiphertextOutputPublished,
        _: &mut actix::Context<Self>,
    ) -> Self::Result {
        let fhe = self.fhe.clone();
        let data = self.data.clone();
        let bus = self.bus.clone();
        let address = self.address.clone();
        Box::pin(async move {
            on_decryption_requested(fhe, data, bus, event, address)
                .await
                .unwrap()
        })
    }
}

impl Handler<Die> for Keyshare {
    type Result = ();
    fn handle(&mut self, _: Die, ctx: &mut Self::Context) -> Self::Result {
        ctx.stop()
    }
}

async fn on_decryption_requested(
    fhe: Arc<Fhe>,
    data: DataStore,
    bus: Addr<EventBus>,
    event: CiphertextOutputPublished,
    address: String,
) -> Result<()> {
    let CiphertextOutputPublished {
        e3_id,
        ciphertext_output,
    } = event;

    // get secret key by id from data
    let Some(unsafe_secret) = data.read(Get(format!("{}/sk", e3_id).into())).await? else {
        return Err(anyhow::anyhow!("Secret key not stored for {}", e3_id));
    };

    println!("\n\nDECRYPTING!\n\n");

    let decryption_share = fhe
        .decrypt_ciphertext(DecryptCiphertext {
            ciphertext: ciphertext_output,
            unsafe_secret,
        })
        .context("error decrypting ciphertext")?;

    let event = EnclaveEvent::from(DecryptionshareCreated {
        e3_id,
        decryption_share,
        node: address,
    });

    bus.do_send(event);

    Ok(())
}
