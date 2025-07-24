// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{E3Context, E3ContextSnapshot, E3Extension, MetaRepositoryFactory, TypedKey};
use anyhow::*;
use async_trait::async_trait;
use e3_data::RepositoriesFactory;
use e3_events::{E3Requested, EnclaveEvent, Seed};

pub const META_KEY: TypedKey<E3Meta> = TypedKey::new("meta");

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct E3Meta {
    pub threshold_m: usize,
    pub seed: Seed,
}

pub struct E3MetaExtension;

impl E3MetaExtension {
    pub fn create() -> Box<Self> {
        Box::new(Self {})
    }
}

#[async_trait]
impl E3Extension for E3MetaExtension {
    fn on_event(&self, ctx: &mut crate::E3Context, event: &EnclaveEvent) {
        let EnclaveEvent::E3Requested { data, .. } = event else {
            return;
        };
        let E3Requested {
            threshold_m,
            seed,
            e3_id,
            ..
        } = data.clone();

        // Meta doesn't implement Checkpoint so we are going to store it manually
        let meta = E3Meta { threshold_m, seed };
        ctx.repositories().meta(&e3_id).write(&meta);
        let _ = ctx.set_dependency(META_KEY, meta);
    }

    async fn hydrate(&self, ctx: &mut E3Context, snapshot: &E3ContextSnapshot) -> Result<()> {
        // No ID on the snapshot -> bail
        if !snapshot.contains("meta") {
            return Ok(());
        };

        let repository = ctx.repositories().meta(&ctx.e3_id);

        // No Snapshot returned from the store -> bail
        let Some(value) = repository.read().await? else {
            return Ok(());
        };

        ctx.set_dependency(META_KEY, value);

        Ok(())
    }
}
