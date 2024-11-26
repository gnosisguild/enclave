use crate::{E3Feature, E3RequestContext, E3RequestContextSnapshot, MetaRepositoryFactory};
use data::RepositoriesFactory;
use anyhow::*;
use async_trait::async_trait;
use enclave_core::{E3Requested, EnclaveEvent, Seed};

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CommitteeMeta {
    pub threshold_m: usize,
    pub seed: Seed,
    pub src_chain_id: u64,
}

pub struct CommitteeMetaFeature;

impl CommitteeMetaFeature {
    pub fn create() -> Box<Self> {
        Box::new(Self {})
    }
}

#[async_trait]
impl E3Feature for CommitteeMetaFeature {
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
        let meta = CommitteeMeta {
            threshold_m,
            seed,
            src_chain_id,
        };
        ctx.repositories().meta(&e3_id).write(&meta);
        let _ = ctx.set_meta(meta);
    }

    async fn hydrate(
        &self,
        ctx: &mut E3RequestContext,
        snapshot: &E3RequestContextSnapshot,
    ) -> Result<()> {
        // No ID on the snapshot -> bail
        if !snapshot.meta {
            return Ok(());
        };

        let repository = ctx.repositories().meta(&ctx.e3_id);

        // No Snapshot returned from the store -> bail
        let Some(value) = repository.read().await? else {
            return Ok(());
        };

        ctx.set_meta(value);

        Ok(())
    }
}

