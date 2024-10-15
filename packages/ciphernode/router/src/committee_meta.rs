use crate::{E3Feature, E3RequestContext, E3RequestContextSnapshot};
use anyhow::*;
use async_trait::async_trait;
use data::WithPrefix;
use enclave_core::{E3Requested, EnclaveEvent, Seed};

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CommitteeMeta {
    pub threshold_m: usize,
    pub seed: Seed,
    pub src_chain_id: u64,
}

pub struct CommitteMetaFeature;

impl CommitteMetaFeature {
    pub fn create() -> Box<Self> {
        Box::new(Self {})
    }
}

#[async_trait]
impl E3Feature for CommitteMetaFeature {
    fn on_event(&self, ctx: &mut crate::E3RequestContext, event: &EnclaveEvent) {
        let EnclaveEvent::E3Requested { data, .. } = event else {
            return;
        };
        let E3Requested {
            threshold_m,
            seed,
            src_chain_id,
            e3_id,
            ..
        } = data.clone();

        // Meta doesn't implement Checkpoint so we are going to store it manually
        let meta_id = format!("//meta/{e3_id}");
        let meta = CommitteeMeta {
            threshold_m,
            seed,
            src_chain_id,
        };
        ctx.get_store().at(&meta_id).write(meta.clone());
        let _ = ctx.set_meta(meta);
    }

    async fn hydrate(
        &self,
        ctx: &mut E3RequestContext,
        snapshot: &E3RequestContextSnapshot,
    ) -> Result<()> {
        // No ID on the snapshot -> bail
        let Some(id) = snapshot.meta.clone() else {
            return Ok(());
        };

        // No Snapshot returned from the store -> bail
        let Some(value) = ctx.store.read_at(&id).await? else {
            return Ok(());
        };

        ctx.set_meta(value);

        Ok(())
    }
}
