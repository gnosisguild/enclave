use std::{collections::BTreeMap, ops::Deref};

use actix::Handler;
use e3_utils::Responder;

use crate::{Insert, SeekForPrev, SeekForPrevResponse, SeekableStore};

pub struct InMemHlcStore {
    db: InMemDb,
}

struct InMemDb(BTreeMap<Vec<u8>, Vec<u8>>);
impl Deref for InMemDb {
    type Target = BTreeMap<Vec<u8>, Vec<u8>>;
    fn deref(&self) -> &Self::Target {}
}

impl SeekableStore for InMemDb {
    fn seek_for_prev(&self, msg: SeekForPrev) -> Result<Option<Vec<u8>>> {
        let key = msg.key();
        let entry = self.range(..=&key[..]).next_back();

        match entry {
            Some(Ok((_, bytes))) => Ok(Some(bytes.as_ref().try_into()?)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }
}

impl Handler<Responder<SeekForPrev, SeekForPrevResponse>> for InMemHlcStore {
    type Result = ();
    fn handle(
        &mut self,
        msg: Responder<SeekForPrev, SeekForPrevResponse>,
        _: &mut Self::Context,
    ) -> Self::Result {
        trap(e3_events::EType::Data, &self.bus.clone(), || {
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
            self.db.insert(msg.key(), msg.value().to_vec())?;
            Ok(())
        })
    }
}
