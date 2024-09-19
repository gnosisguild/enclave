use crate::{ActorFactory, CommitteeRequested, EnclaveEvent};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitteeMeta {
    pub nodecount: usize,
    pub seed: u64,
}

pub struct CommitteeMetaFactory;

impl CommitteeMetaFactory {
    pub fn create() -> ActorFactory {
        Box::new(move |ctx, evt| {
            let EnclaveEvent::CommitteeRequested { data, .. }: crate::EnclaveEvent = evt else {
                return;
            };
            let CommitteeRequested {
                nodecount,
                sortition_seed: seed,
                ..
            } = data;

            ctx.meta = Some(CommitteeMeta { nodecount, seed });
        })
    }
}
