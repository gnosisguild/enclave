use enclave_core::{E3Requested, EnclaveEvent, Seed};

use super::EventHook;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CommitteeMeta {
    pub threshold_m: usize,
    pub seed: Seed,
    pub src_chain_id: u64,
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
                src_chain_id,
                e3_id,
                ..
            } = data;

            ctx.set_meta(
                &format!("//meta/{e3_id}"),
                CommitteeMeta {
                    threshold_m,
                    seed,
                    src_chain_id,
                },
            );
        })
    }
}
