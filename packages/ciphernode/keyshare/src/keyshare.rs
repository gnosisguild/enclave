use actix::prelude::*;
use anyhow::anyhow;
use data::{DataStore, Get, Insert};
use enclave_core::{
    CiphernodeSelected, CiphertextOutputPublished, DecryptionshareCreated, Die, EnclaveErrorType,
    EnclaveEvent, EventBus, FromError, KeyshareCreated,
};
use fhe::{DecryptCiphertext, Fhe};
use std::sync::Arc;
use tracing::error;

pub struct Keyshare {
    fhe: Arc<Fhe>,
    /// Data must be prefixed correctly to the actor's namespace.
    data: DataStore,
    /// The Keyshares Secret (This is currently unencrypted but eventually this will be encrypted)
    secret: Option<Vec<u8>>,
    bus: Addr<EventBus>,
    address: String,
}

impl Actor for Keyshare {
    type Context = actix::Context<Self>;
}

impl Keyshare {
    pub fn new(
        bus: Addr<EventBus>,
        data: DataStore,
        secret: Option<Vec<u8>>,
        fhe: Arc<Fhe>,
        address: &str,
    ) -> Self {
        Self {
            bus,
            fhe,
            data,
            secret,
            address: address.to_string(),
        }
    }

    pub async fn hydrate(
        bus: Addr<EventBus>,
        data: DataStore,
        fhe: Arc<Fhe>,
        address: &str,
    ) -> Addr<Keyshare> {
        let secret = data.read(Get::new("secret")).await.unwrap_or_else(|err| {
            error!("Could not retrieve secret from data store: {err}");
            None
        });

        Keyshare::new(bus, data, secret, fhe, address).start()
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

        self.secret = Some(sk.clone());

        // TODO: encrypt
        self.data.write(Insert::new("secret", sk));

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

        // get secret key by id from data
        let Some(unsafe_secret) = &self.secret else {
            return error!("Secret key not stored for {}", e3_id);
        };

        println!("\n\nDECRYPTING!\n\n");

        let Ok(decryption_share) = self.fhe.decrypt_ciphertext(DecryptCiphertext {
            ciphertext: ciphertext_output,
            unsafe_secret: unsafe_secret.to_vec(),
        }) else {
            error!("error decrypting ciphertext");
            return;
        };

        let event = EnclaveEvent::from(DecryptionshareCreated {
            e3_id,
            decryption_share,
            node: self.address.clone(),
        });

        println!("DECRYPTIONSHARE");

        self.bus.do_send(event);
    }
}

impl Handler<Die> for Keyshare {
    type Result = ();
    fn handle(&mut self, _: Die, ctx: &mut Self::Context) -> Self::Result {
        ctx.stop()
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{Arc, Mutex},
        time::Duration,
    };

    use actix::Actor;
    use anyhow::Result;
    use data::{DataStore, GetLog, InMemDataStore, Insert};
    use enclave_core::{
        CiphertextOutputPublished, DecryptionshareCreated, E3id, EnclaveEvent, EventBus, GetHistory,
    };
    use fhe::{setup_bfv_encoded, Fhe};
    use fhe_rs::{
        bfv::{Ciphertext, Encoding, Plaintext, PublicKey},
        mbfv::{AggregateIter, PublicKeyShare},
    };
    use fhe_traits::{FheEncoder, FheEncrypter, Serialize};
    use rand::{self, SeedableRng};
    use rand_chacha::ChaCha20Rng;
    use tokio::time::sleep;

    use super::Keyshare;

    #[actix::test]
    async fn test_hydration() -> Result<()> {
        // Normally I would just add an event to check the state but I don't want to allow that as
        // this should not expose it's secret to the outside world so instead I am simulating a
        // keyshare decryption

        // Setup
        let store = DataStore::from_in_mem(InMemDataStore::new(false).start());
        let bus = EventBus::new(true).start();
        let (params_bytes, seed, rng) = setup_bfv_encoded(
            &[0x3FFFFFFF000001],
            2048,
            1032193,
            ChaCha20Rng::seed_from_u64(123),
        );

        // We haven't setup traits to test the fhe lib for a unit test so we need to create a real
        // instance unfortnuately
        let fhe = Arc::new(Fhe::from_encoded(&params_bytes, seed, rng)?);
        let fhe_test = Arc::new(Fhe::from_encoded(
            &params_bytes,
            seed,
            Arc::new(Mutex::new(ChaCha20Rng::seed_from_u64(123))),
        )?);

        let (sk, pks) = fhe.generate_keyshare()?;
        let _ = fhe_test.generate_keyshare()?; // keep rngs in sync - might be a better way to do
                                               // this?

        let e3_id = E3id::new("1234");

        // Write the secret to the db
        store.write(Insert::new("secret", sk.clone()));
        let address = "0x0000000000000000000000000000000000000000";

        // Hydrate the keystore
        let ks = Keyshare::hydrate(bus.clone(), store, fhe.clone(), address).await;

        // Setup some FHE apparatus to chec a deserialization
        let pks = PublicKeyShare::deserialize(&pks, &fhe.params, fhe.crp.clone());
        let pubkey: PublicKey = vec![pks].into_iter().aggregate()?;
        let ct = pubkey.try_encrypt(
            &Plaintext::try_encode(&[12u64, 34u64, 56u64, 78u64], Encoding::poly(), &fhe.params)?,
            &mut ChaCha20Rng::seed_from_u64(123),
        )?;

        ks.do_send(EnclaveEvent::from(CiphertextOutputPublished {
            ciphertext_output: ct.to_bytes(),
            e3_id: e3_id.clone(),
        }));

        // Expect the correct decryption
        let expected = DecryptionshareCreated {
            node: address.to_string(),
            e3_id: e3_id.clone(),
            decryption_share: fhe_test.decrypt_ciphertext(fhe::DecryptCiphertext {
                unsafe_secret: sk,
                ciphertext: ct.to_bytes(),
            })?,
        };

        sleep(Duration::from_millis(1)).await;

        let history = bus.send(GetHistory).await?;
        let EnclaveEvent::DecryptionshareCreated { data, .. } = history[0].clone() else {
            panic!("Event should be DecryptionshareCreated!");
        };

        assert_eq!(data.decryption_share, expected.decryption_share);

        Ok(())
    }
}
