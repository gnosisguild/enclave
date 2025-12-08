use crate::{Insert, KeyValStore, SeekForPrev, SeekForPrevResponse, SeekableStore, SledDb};
use actix::{Actor, Handler};
use e3_events::{trap, BusHandle};
use e3_utils::actix::Responder;

pub struct HlcStore {
    db: SledDb,
    bus: BusHandle,
}

impl HlcStore {
    pub fn new(db: SledDb, bus: &BusHandle) -> Self {
        Self {
            db,
            bus: bus.clone(),
        }
    }
}

impl Actor for HlcStore {
    type Context = actix::Context<Self>;
}

impl Handler<Responder<SeekForPrev, SeekForPrevResponse>> for HlcStore {
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

impl Handler<Insert> for HlcStore {
    type Result = ();
    fn handle(&mut self, msg: Insert, _: &mut Self::Context) -> Self::Result {
        trap(e3_events::EType::Data, &self.bus.clone(), || {
            self.db.insert(msg)?;
            Ok(())
        })
    }
}
