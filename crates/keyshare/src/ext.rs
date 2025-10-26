// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{
    Keyshare, KeyshareParams, KeyshareRepositoryFactory, KeyshareState, ThresholdKeyshare,
    ThresholdKeyshareParams, ThresholdKeyshareRepositoryFactory, ThresholdKeyshareState,
};
use actix::{Actor, Addr};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use e3_crypto::Cipher;
use e3_data::{AutoPersist, RepositoriesFactory};
use e3_events::{BusError, CiphernodeSelected, EnclaveErrorType, EnclaveEvent, EventBus};
use e3_fhe::ext::FHE_KEY;
use e3_multithread::Multithread;
use e3_request::{E3Context, E3ContextSnapshot, E3Extension, TypedKey, META_KEY};
use std::sync::Arc;

const CIPHERNODE_SELECTED_KEY: TypedKey<CiphernodeSelected> = TypedKey::new("ciphernode_selected");

pub struct KeyshareExtension {
    bus: Addr<EventBus<EnclaveEvent>>,
    address: String,
    cipher: Arc<Cipher>,
}

impl KeyshareExtension {
    pub fn create(
        bus: &Addr<EventBus<EnclaveEvent>>,
        address: &str,
        cipher: &Arc<Cipher>,
    ) -> Box<Self> {
        Box::new(Self {
            bus: bus.clone(),
            address: address.to_owned(),
            cipher: cipher.to_owned(),
        })
    }
}

const ERROR_KEYSHARE_FHE_MISSING: &str =
    "Could not create Keyshare because the fhe instance it depends on was not set on the context.";

#[async_trait]
impl E3Extension for KeyshareExtension {
    fn on_event(&self, ctx: &mut E3Context, evt: &EnclaveEvent) {
        match evt {
            // Store CiphernodeSelected data for later use
            EnclaveEvent::CiphernodeSelected { data, .. } => {
                // For score sortition, CiphernodeSelected just means we might be selected
                // We need to wait for CommitteeFinalized to confirm we're actually in the committee
                // Store the selection data for when CommitteeFinalized arrives
                if data.ticket_id.is_some() {
                    // Store selection data - we'll start keyshare generation after CommitteeFinalized
                    ctx.set_dependency(CIPHERNODE_SELECTED_KEY, data.clone());
                    return;
                }

                // For distance sortition (no ticket_id), proceed immediately as before
                // Has the FHE dependency been already setup? (hint: it should have)
                let Some(fhe) = ctx.get_dependency(FHE_KEY) else {
                    self.bus.err(
                        EnclaveErrorType::KeyGeneration,
                        anyhow!(ERROR_KEYSHARE_FHE_MISSING),
                    );
                    return;
                };

                let e3_id = data.clone().e3_id;
                let repo = ctx.repositories().keyshare(&e3_id);
                let container = repo.send(None); // New container with None

                ctx.set_event_recipient(
                    "keyshare",
                    Some(
                        Keyshare::new(KeyshareParams {
                            bus: self.bus.clone(),
                            secret: container,
                            fhe: fhe.clone(),
                            address: self.address.clone(),
                            cipher: self.cipher.clone(),
                        })
                        .start()
                        .into(),
                    ),
                );
            }
            // For score sortition, start keyshare generation after CommitteeFinalized
            EnclaveEvent::CommitteeFinalized { data, .. } => {
                // Check if we have stored CiphernodeSelected data (score sortition)
                let Some(selected_data) = ctx.get_dependency(CIPHERNODE_SELECTED_KEY) else {
                    // No stored data means this was distance sortition or we weren't selected
                    return;
                };

                // Verify this node is in the finalized committee
                if !data.committee.contains(&self.address) {
                    // We submitted a ticket but didn't make it into the final committee
                    return;
                }

                // Has the FHE dependency been already setup? (hint: it should have)
                let Some(fhe) = ctx.get_dependency(FHE_KEY) else {
                    self.bus.err(
                        EnclaveErrorType::KeyGeneration,
                        anyhow!(ERROR_KEYSHARE_FHE_MISSING),
                    );
                    return;
                };

                let e3_id = selected_data.e3_id.clone();
                let repo = ctx.repositories().keyshare(&e3_id);
                let container = repo.send(None); // New container with None

                ctx.set_event_recipient(
                    "keyshare",
                    Some(
                        Keyshare::new(KeyshareParams {
                            bus: self.bus.clone(),
                            secret: container,
                            fhe: fhe.clone(),
                            address: self.address.clone(),
                            cipher: self.cipher.clone(),
                        })
                        .start()
                        .into(),
                    ),
                );
            }
            _ => {}
        }
    }

    async fn hydrate(&self, ctx: &mut E3Context, snapshot: &E3ContextSnapshot) -> Result<()> {
        // No keyshare on the snapshot -> bail
        if !snapshot.contains("keyshare") {
            return Ok(());
        };

        // Get the saved state as a persistable
        let sync_secret = ctx.repositories().keyshare(&snapshot.e3_id).load().await?;

        // No Snapshot returned from the sync_secret -> bail
        if !sync_secret.has() {
            return Ok(());
        };

        // Has the FHE dependency been already setup? (hint: it should have)
        let Some(fhe) = ctx.get_dependency(FHE_KEY) else {
            self.bus.err(
                EnclaveErrorType::KeyGeneration,
                anyhow!(ERROR_KEYSHARE_FHE_MISSING),
            );
            return Ok(());
        };

        // Construct from snapshot
        let value = Keyshare::new(KeyshareParams {
            fhe: fhe.clone(),
            bus: self.bus.clone(),
            secret: sync_secret,
            address: self.address.clone(),
            cipher: self.cipher.clone(),
        })
        .start()
        .into();

        // send to context
        ctx.set_event_recipient("keyshare", Some(value));

        Ok(())
    }
}

pub struct ThresholdKeyshareExtension {
    bus: Addr<EventBus<EnclaveEvent>>,
    cipher: Arc<Cipher>,
    address: String,
    multithread: Addr<Multithread>,
}

impl ThresholdKeyshareExtension {
    pub fn create(
        bus: &Addr<EventBus<EnclaveEvent>>,
        cipher: &Arc<Cipher>,
        multithread: &Addr<Multithread>,
        address: &str,
    ) -> Box<Self> {
        Box::new(Self {
            bus: bus.clone(),
            cipher: cipher.to_owned(),
            multithread: multithread.clone(),
            address: address.to_owned(),
        })
    }
}

#[async_trait]
impl E3Extension for ThresholdKeyshareExtension {
    fn on_event(&self, ctx: &mut E3Context, evt: &EnclaveEvent) {
        match evt {
            // Store CiphernodeSelected data for later use
            EnclaveEvent::CiphernodeSelected { data, .. } => {
                // For score sortition, CiphernodeSelected just means we might be selected
                // We need to wait for CommitteeFinalized to confirm we're actually in the committee
                // Store the selection data for when CommitteeFinalized arrives
                if data.ticket_id.is_some() {
                    // Store selection data - we'll start keyshare generation after CommitteeFinalized
                    ctx.set_dependency(CIPHERNODE_SELECTED_KEY, data.clone());
                    return;
                }

                // For distance sortition (no ticket_id), proceed immediately as before
                let e3_id = data.clone().e3_id;
                let party_id = data.clone().party_id;
                let Some(meta) = ctx.get_dependency(META_KEY) else {
                    self.bus.err(
                        EnclaveErrorType::KeyGeneration,
                        anyhow!(ERROR_KEYSHARE_FHE_MISSING),
                    );
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

                ctx.set_event_recipient(
                    "threshold_keyshare",
                    Some(
                        ThresholdKeyshare::new(ThresholdKeyshareParams {
                            bus: self.bus.clone(),
                            cipher: self.cipher.clone(),
                            multithread: self.multithread.clone(),
                            state: container,
                        })
                        .start()
                        .into(),
                    ),
                );
            }
            // For score sortition, start keyshare generation after CommitteeFinalized
            EnclaveEvent::CommitteeFinalized { data, .. } => {
                // Check if we have stored CiphernodeSelected data (score sortition)
                let Some(selected_data) = ctx.get_dependency(CIPHERNODE_SELECTED_KEY) else {
                    // No stored data means this was distance sortition or we weren't selected
                    return;
                };

                // Verify this node is in the finalized committee
                if !data.committee.contains(&self.address) {
                    // We submitted a ticket but didn't make it into the final committee
                    return;
                }

                let e3_id = selected_data.e3_id.clone();
                let party_id = selected_data.party_id;
                let Some(meta) = ctx.get_dependency(META_KEY) else {
                    self.bus.err(
                        EnclaveErrorType::KeyGeneration,
                        anyhow!(ERROR_KEYSHARE_FHE_MISSING),
                    );
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

                ctx.set_event_recipient(
                    "threshold_keyshare",
                    Some(
                        ThresholdKeyshare::new(ThresholdKeyshareParams {
                            bus: self.bus.clone(),
                            cipher: self.cipher.clone(),
                            multithread: self.multithread.clone(),
                            state: container,
                        })
                        .start()
                        .into(),
                    ),
                );
            }
            _ => {}
        }
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
            multithread: self.multithread.clone(),
            state,
        })
        .start()
        .into();

        // send to context
        ctx.set_event_recipient("threshold_keyshare", Some(value));

        Ok(())
    }
}
