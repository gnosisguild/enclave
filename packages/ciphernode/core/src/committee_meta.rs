use crate::{ActorFactory, E3Requested, EnclaveEvent};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitteeMeta {
    pub threshold_m: u32,
    pub seed: u64,
}

pub struct CommitteeMetaFactory;

impl CommitteeMetaFactory {
    pub fn create() -> ActorFactory {
        Box::new(move |ctx, evt| {
            let EnclaveEvent::E3Requested { data, .. }: crate::EnclaveEvent = evt else {
                return;
            };
            let E3Requested {
                threshold_m,
                seed: seed,
                ..
            } = data;

            ctx.meta = Some(CommitteeMeta { threshold_m, seed });
        })
    }
}
