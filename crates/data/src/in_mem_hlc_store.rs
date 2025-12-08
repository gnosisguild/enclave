use std::{collections::BTreeMap, ops::Deref};

use crate::{Get, Insert, KeyValStore, SeekForPrev, SeekForPrevResponse, SeekableStore};
use actix::{Actor, Handler};
use anyhow::Result;
use e3_events::{trap, BusHandle};
use e3_utils::Responder;

pub struct InMemHlcStore {
    db: InMemDb,
    bus: BusHandle,
}

impl Actor for InMemHlcStore {
    type Context = actix::Context<Self>;
}

struct InMemDb(BTreeMap<Vec<u8>, Vec<u8>>);
impl Deref for InMemDb {
    type Target = BTreeMap<Vec<u8>, Vec<u8>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl KeyValStore for InMemDb {
    fn get(&self, msg: Get) -> Result<Option<Vec<u8>>> {
        Ok(self.0.get(msg.key()).cloned())
    }
    fn insert(&mut self, msg: Insert) -> Result<()> {
        self.0.insert(msg.key().to_owned(), msg.value().to_owned());
        Ok(())
    }
    fn remove(&mut self, msg: crate::Remove) -> Result<()> {
        self.0.remove(msg.key());
        Ok(())
    }
}

impl SeekableStore for InMemDb {
    fn seek_for_prev(&self, msg: SeekForPrev) -> Result<Option<Vec<u8>>> {
        let key = msg.key();
        Ok(self
            .0
            .range(..=key.to_vec())
            .next_back()
            .map(|(_, v)| v.clone()))
    }
}

impl Handler<Responder<SeekForPrev, SeekForPrevResponse>> for InMemHlcStore {
    type Result = ();
    fn handle(
        &mut self,
        msg: Responder<SeekForPrev, SeekForPrevResponse>,
        _: &mut Self::Context,
    ) -> Self::Result {
        trap(e3_events::EType::Data, &self.bus, || {
            let Some(result) = self.db.seek_for_prev((*msg).clone())? else {
                return Err(anyhow::anyhow!("Seek returned no result."));
            };

            let seq_bytes: [u8; 8] = result[..8]
                .try_into()
                .expect("sequence must be exactly 8 bytes");

            let seq = u64::from_be_bytes(seq_bytes);
            msg.try_reply(SeekForPrevResponse::new(seq))?;
            Ok(())
        })
    }
}

impl Handler<Insert> for InMemHlcStore {
    type Result = ();
    fn handle(&mut self, msg: Insert, _: &mut Self::Context) -> Self::Result {
        trap(e3_events::EType::Data, &self.bus.clone(), || {
            self.db.insert(msg)?;
            Ok(())
        })
    }
}
