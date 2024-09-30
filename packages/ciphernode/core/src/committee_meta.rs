use crate::{ActorFactory, E3Requested, EnclaveEvent, Seed};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitteeMeta {
    pub threshold_m: usize,
    pub seed: Seed,
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
                seed,
                ..
            } = data;

            ctx.meta = Some(CommitteeMeta { threshold_m, seed });
        })
    }
}
