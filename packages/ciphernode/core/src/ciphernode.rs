use crate::{
    data::{Data, Insert},
    eventbus::EventBus,
    events::{EnclaveEvent, KeyshareCreated},
    fhe::{Fhe, GenerateKeyshare},
    ActorFactory, CiphernodeSelected, CiphertextOutputPublished, DecryptCiphertext,
    DecryptionshareCreated, Get,
};
use actix::prelude::*;
use alloy_primitives::Address;
use anyhow::Result;

pub struct Ciphernode {
    fhe: Addr<Fhe>,
    data: Addr<Data>,
    bus: Addr<EventBus>,
    address: Address,
}

impl Actor for Ciphernode {
    type Context = Context<Self>;
}

impl Ciphernode {
    pub fn new(bus: Addr<EventBus>, fhe: Addr<Fhe>, data: Addr<Data>, address: Address) -> Self {
        Self {
            bus,
            fhe,
            data,
            address,
        }
    }
}

impl Handler<EnclaveEvent> for Ciphernode {
    type Result = ();

    fn handle(&mut self, event: EnclaveEvent, ctx: &mut Context<Self>) -> Self::Result {
        match event {
            EnclaveEvent::CiphernodeSelected { data, .. } => ctx.address().do_send(data),
            EnclaveEvent::CiphertextOutputPublished { data, .. } => ctx.address().do_send(data),
            _ => (),
        }
    }
}

impl Handler<CiphernodeSelected> for Ciphernode {
    type Result = ResponseFuture<()>;

    fn handle(&mut self, event: CiphernodeSelected, _: &mut Context<Self>) -> Self::Result {
        let fhe = self.fhe.clone();
        let data = self.data.clone();
        let bus = self.bus.clone();
        let address = self.address;
        Box::pin(async move {
            on_ciphernode_selected(fhe, data, bus, event, address)
                .await
                .unwrap()
        })
    }
}

impl Handler<CiphertextOutputPublished> for Ciphernode {
    type Result = ResponseFuture<()>;

    fn handle(&mut self, event: CiphertextOutputPublished, _: &mut Context<Self>) -> Self::Result {
        println!("Ciphernode::CiphertextOutputPublished");
        let fhe = self.fhe.clone();
        let data = self.data.clone();
        let bus = self.bus.clone();
        let address = self.address;
        Box::pin(async move {
            on_decryption_requested(fhe, data, bus, event, address)
                .await
                .unwrap()
        })
    }
}

async fn on_ciphernode_selected(
    fhe: Addr<Fhe>,
    data: Addr<Data>,
    bus: Addr<EventBus>,
    event: CiphernodeSelected,
    address: Address,
) -> Result<()> {
    let CiphernodeSelected { e3_id, .. } = event;

    println!("\n\nGENERATING KEY!\n\n");

    // generate keyshare
    let (sk, pubkey) = fhe.send(GenerateKeyshare {}).await??;

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
    fhe: Addr<Fhe>,
    data: Addr<Data>,
    bus: Addr<EventBus>,
    event: CiphertextOutputPublished,
    address: Address,
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

    let decryption_share = fhe
        .send(DecryptCiphertext {
            ciphertext: ciphertext_output,
            unsafe_secret,
        })
        .await??;

    let event = EnclaveEvent::from(DecryptionshareCreated {
        e3_id,
        decryption_share,
        node: address,
    });

    bus.do_send(event);

    Ok(())
}

pub struct CiphernodeFactory;
impl CiphernodeFactory {
    pub fn create(bus: Addr<EventBus>, data: Addr<Data>, address: Address) -> ActorFactory {
        Box::new(move |ctx, evt| {
            // Save Ciphernode on CiphernodeSelected
            let EnclaveEvent::CiphernodeSelected { .. } = evt else {
                return;
            };

            let Some(ref fhe) = ctx.fhe else {
                return;
            };

            ctx.ciphernode =
                Some(Ciphernode::new(bus.clone(), fhe.clone(), data.clone(), address).start())
        })
    }
}
