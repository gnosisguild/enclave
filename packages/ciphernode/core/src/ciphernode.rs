use std::sync::Arc;

use crate::{
    data::{Data, Insert},
    eventbus::EventBus,
    events::{EnclaveEvent, KeyshareCreated},
    fhe::Fhe,
    ActorFactory, CiphernodeSelected, CiphertextOutputPublished, DecryptCiphertext,
    DecryptionshareCreated, Get,
};
use actix::prelude::*;
use anyhow::Result;
use anyhow::Context;

pub struct Keyshare {
    fhe: Arc<Fhe>,
    data: Addr<Data>,
    bus: Addr<EventBus>,
    address: String,
}

impl Actor for Keyshare {
    type Context = actix::Context<Self>;
}

impl Keyshare {
    pub fn new(bus: Addr<EventBus>, data: Addr<Data>, fhe: Arc<Fhe>, address: &str) -> Self {
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
            EnclaveEvent::CiphernodeSelected { data, .. } => ctx.address().do_send(data),
            EnclaveEvent::CiphertextOutputPublished { data, .. } => ctx.address().do_send(data),
            _ => (),
        }
    }
}

impl Handler<CiphernodeSelected> for Keyshare {
    type Result = ResponseFuture<()>;

    fn handle(&mut self, event: CiphernodeSelected, _: &mut actix::Context<Self>) -> Self::Result {
        let fhe = self.fhe.clone();
        let data = self.data.clone();
        let bus = self.bus.clone();
        let address = self.address.clone();
        Box::pin(async move {
            on_ciphernode_selected(fhe, data, bus, event, address)
                .await
                .unwrap()
        })
    }
}

impl Handler<CiphertextOutputPublished> for Keyshare {
    type Result = ResponseFuture<()>;

    fn handle(&mut self, event: CiphertextOutputPublished, _: &mut actix::Context<Self>) -> Self::Result {
        println!("Ciphernode::CiphertextOutputPublished");
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

async fn on_ciphernode_selected(
    fhe: Arc<Fhe>,
    data: Addr<Data>,
    bus: Addr<EventBus>,
    event: CiphernodeSelected,
    address: String,
) -> Result<()> {
    let CiphernodeSelected { e3_id, .. } = event;

    // generate keyshare
    let (sk, pubkey) = fhe.generate_keyshare()?;

    // TODO: decrypt from FHE actor
    // save encrypted key against e3_id/sk
    // reencrypt secretkey locally with env var - this is so we don't have to serialize a secret
    // best practice would be as you boot up a node you enter in a configured password from
    // which we derive a kdf which gets used to generate this key
    data.do_send(Insert(format!("{}/sk", e3_id).into(), sk));

    // save public key against e3_id/pk
    data.do_send(Insert(format!("{}/pk", e3_id).into(), pubkey.clone()));

    // broadcast the KeyshareCreated message
    let event = EnclaveEvent::from(KeyshareCreated {
        pubkey,
        e3_id,
        node: address,
    });
    bus.do_send(event);

    Ok(())
}

async fn on_decryption_requested(
    fhe: Arc<Fhe>,
    data: Addr<Data>,
    bus: Addr<EventBus>,
    event: CiphertextOutputPublished,
    address: String,
) -> Result<()> {
    let CiphertextOutputPublished {
        e3_id,
        ciphertext_output,
    } = event;

    // get secret key by id from data
    let Some(unsafe_secret) = data.send(Get(format!("{}/sk", e3_id).into())).await? else {
        return Err(anyhow::anyhow!("Secret key not stored for {}", e3_id));
    };

    println!("\n\nDECRYPTING!\n\n");

    let decryption_share = fhe.decrypt_ciphertext(DecryptCiphertext {
        ciphertext: ciphertext_output,
        unsafe_secret,
    }).context("error decrypting ciphertext")?;

    let event = EnclaveEvent::from(DecryptionshareCreated {
        e3_id,
        decryption_share,
        node: address,
    });

    bus.do_send(event);

    Ok(())
}

pub struct KeyshareFactory;
impl KeyshareFactory {
    pub fn create(bus: Addr<EventBus>, data: Addr<Data>, address: &str) -> ActorFactory {
        let address = address.to_string();
        Box::new(move |ctx, evt| {
            // Save Ciphernode on CiphernodeSelected
            let EnclaveEvent::CiphernodeSelected { .. } = evt else {
                return;
            };

            let Some(ref fhe) = ctx.fhe else {
                return;
            };

            ctx.keyshare =
                Some(Keyshare::new(bus.clone(), data.clone(), fhe.clone(), &address).start())
        })
    }
}
