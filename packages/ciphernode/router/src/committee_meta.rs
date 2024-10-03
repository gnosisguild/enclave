use enclave_core::{E3Requested, EnclaveEvent, Seed};

use super::EventHook;


#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitteeMeta {
    pub threshold_m: usize,
    pub seed: Seed,
}

pub struct CommitteeMetaFactory;

impl CommitteeMetaFactory {
    pub fn create() -> EventHook {
        Box::new(move |ctx, evt| {
            let EnclaveEvent::E3Requested { data, .. } = evt else {
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
