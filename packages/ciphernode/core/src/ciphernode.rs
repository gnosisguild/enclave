use crate::{
    data::{Data, Insert},
    eventbus::EventBus,
    events::{CommitteeRequested, EnclaveEvent, KeyshareCreated},
    fhe::{Fhe, GenerateKeyshare},
    CiphertextOutputPublished, DecryptCiphertext, DecryptionshareCreated, Get, Subscribe,
};
use actix::prelude::*;
use anyhow::Result;

pub struct Ciphernode {
    fhe: Addr<Fhe>,
    data: Addr<Data>,
    bus: Addr<EventBus>,
}

impl Actor for Ciphernode {
    type Context = Context<Self>;
}

impl Ciphernode {
    pub fn new(bus: Addr<EventBus>, fhe: Addr<Fhe>, data: Addr<Data>) -> Self {
        Self { bus, fhe, data }
    }

    pub async fn attach(bus: Addr<EventBus>, fhe: Addr<Fhe>, data: Addr<Data>) -> Addr<Self> {
        let node = Ciphernode::new(bus.clone(), fhe, data).start();
        let _ = bus
            .send(Subscribe::new("CommitteeRequested", node.clone().into()))
            .await;
        let _ = bus
            .send(Subscribe::new(
                "CiphertextOutputPublished",
                node.clone().into(),
            ))
            .await;
        node
    }
}

impl Handler<EnclaveEvent> for Ciphernode {
    type Result = ();

    fn handle(&mut self, event: EnclaveEvent, ctx: &mut Context<Self>) -> Self::Result {
        match event {
            EnclaveEvent::CommitteeRequested { data, .. } => ctx.address().do_send(data),
            EnclaveEvent::CiphertextOutputPublished { data, .. } => ctx.address().do_send(data),
            _ => (),
        }
    }
}

impl Handler<CommitteeRequested> for Ciphernode {
    type Result = ResponseFuture<()>;

    fn handle(&mut self, event: CommitteeRequested, _: &mut Context<Self>) -> Self::Result {
        let fhe = self.fhe.clone();
        let data = self.data.clone();
        let bus = self.bus.clone();
        Box::pin(async { on_committee_requested(fhe, data, bus, event).await.unwrap() })
    }
}

impl Handler<CiphertextOutputPublished> for Ciphernode {
    type Result = ResponseFuture<()>;

    fn handle(&mut self, event: CiphertextOutputPublished, _: &mut Context<Self>) -> Self::Result {
        let fhe = self.fhe.clone();
        let data = self.data.clone();
        let bus = self.bus.clone();
        Box::pin(async {
            on_decryption_requested(fhe, data, bus, event)
                .await
                .unwrap()
        })
    }
}

async fn on_committee_requested(
    fhe: Addr<Fhe>,
    data: Addr<Data>,
    bus: Addr<EventBus>,
    event: CommitteeRequested,
) -> Result<()> {
    let CommitteeRequested { e3_id, .. } = event;
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
    let event = EnclaveEvent::from(KeyshareCreated { pubkey, e3_id });
    bus.do_send(event);

    Ok(())
}

async fn on_decryption_requested(
    fhe: Addr<Fhe>,
    data: Addr<Data>,
    bus: Addr<EventBus>,
    event: CiphertextOutputPublished,
) -> Result<()> {
    let CiphertextOutputPublished {
        e3_id,
        ciphertext_output,
    } = event;

    // get secret key by id from data
    let Some(unsafe_secret) = data.send(Get(format!("{}/sk", e3_id).into())).await? else {
        return Err(anyhow::anyhow!("Secret key not stored for {}", e3_id));
    };

    let decryption_share = fhe
        .send(DecryptCiphertext {
            ciphertext: ciphertext_output,
            unsafe_secret,
        })
        .await??;

    let event = EnclaveEvent::from(DecryptionshareCreated {
        e3_id,
        decryption_share,
    });

    bus.do_send(event);

    Ok(())
}