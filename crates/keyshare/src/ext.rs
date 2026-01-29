// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{
    ThresholdKeyshare, ThresholdKeyshareParams, ThresholdKeyshareRepositoryFactory,
    ThresholdKeyshareState,
};
use actix::Actor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use e3_crypto::Cipher;
use e3_data::{AutoPersist, RepositoriesFactory};
use e3_events::{prelude::*, BusHandle, EType, EnclaveEvent, EnclaveEventData};
use e3_request::{E3Context, E3ContextSnapshot, E3Extension, META_KEY};
use e3_zk_prover::ZkBackend;
use std::sync::Arc;

use crate::KeyshareState;

pub struct ThresholdKeyshareExtension {
    bus: BusHandle,
    cipher: Arc<Cipher>,
    address: String,
    share_encryption_params: Arc<fhe::bfv::BfvParameters>,
    zk_backend: Option<ZkBackend>,
}

impl ThresholdKeyshareExtension {
    pub fn create(
        bus: &BusHandle,
        cipher: &Arc<Cipher>,
        address: &str,
        share_encryption_params: Arc<fhe::bfv::BfvParameters>,
        zk_backend: Option<ZkBackend>,
    ) -> Box<Self> {
        Box::new(Self {
            bus: bus.clone(),
            cipher: cipher.to_owned(),
            address: address.to_owned(),
            share_encryption_params,
            zk_backend,
        })
    }
}

const ERROR_KEYSHARE_META_MISSING: &str =
    "Could not create ThresholdKeyshare because the meta instance it depends on was not set on the context.";

#[async_trait]
impl E3Extension for ThresholdKeyshareExtension {
    fn on_event(&self, ctx: &mut E3Context, evt: &EnclaveEvent) {
        // if this is NOT a CiphernodeSelected event then ignore
        let EnclaveEventData::CiphernodeSelected(data) = evt.get_data() else {
            return;
        };

        let e3_id = data.clone().e3_id;
        let party_id = data.clone().party_id;
        let Some(meta) = ctx.get_dependency(META_KEY) else {
            self.bus
                .err(EType::KeyGeneration, anyhow!(ERROR_KEYSHARE_META_MISSING));
            return;
        };
        let repo = ctx.repositories().threshold_keyshare(&e3_id);
        let container = repo.send(Some(ThresholdKeyshareState::new(
            e3_id.clone(),
            party_id,
            KeyshareState::Init,
            meta.threshold_m as u64,
            meta.threshold_n as u64,
            meta.params.clone(),
            self.address.clone(),
        )));

        // New container with None
        ctx.set_event_recipient(
            "threshold_keyshare",
            Some(
                ThresholdKeyshare::new(ThresholdKeyshareParams {
                    bus: self.bus.clone(),
                    cipher: self.cipher.clone(),
                    state: container,
                    share_encryption_params: self.share_encryption_params.clone(),
                    zk_backend: self.zk_backend.clone(),
                })
                .start()
                .into(),
            ),
        );
    }

    async fn hydrate(&self, ctx: &mut E3Context, snapshot: &E3ContextSnapshot) -> Result<()> {
        // No keyshare on the snapshot -> bail
        if !snapshot.contains("threshold_keyshare") {
            return Ok(());
        };
        // Get the saved state as a persistable
        let state = ctx
            .repositories()
            .threshold_keyshare(&snapshot.e3_id)
            .load()
            .await?;

        // No Snapshot returned from the state -> bail
        if !state.has() {
            return Ok(());
        };

        // Construct from snapshot
        let value = ThresholdKeyshare::new(ThresholdKeyshareParams {
            bus: self.bus.clone(),
            cipher: self.cipher.clone(),
            state,
            share_encryption_params: self.share_encryption_params.clone(),
            zk_backend: self.zk_backend.clone(),
        })
        .start()
        .into();

        // send to context
        ctx.set_event_recipient("threshold_keyshare", Some(value));

        Ok(())
    }
}
