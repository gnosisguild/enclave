// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::sync::Arc;

use crate::keyshare_created_filter_buffer::KeyshareCreatedFilterBuffer;
use crate::{
    PlaintextAggregator, PlaintextAggregatorParams, PlaintextAggregatorState,
    PlaintextRepositoryFactory, PublicKeyAggregator, PublicKeyAggregatorParams,
    PublicKeyAggregatorState, PublicKeyRepositoryFactory, ThresholdPlaintextAggregator,
    ThresholdPlaintextAggregatorParams, ThresholdPlaintextAggregatorState,
    TrBfvPlaintextRepositoryFactory,
};
use actix::{Actor, Addr, Recipient};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use e3_data::{AutoPersist, Persistable, RepositoriesFactory};
use e3_events::{prelude::*, E3id};
use e3_events::{BusHandle, EnclaveErrorType, EnclaveEvent, EnclaveEventData};
use e3_fhe::ext::FHE_KEY;
use e3_fhe::Fhe;
use e3_multithread::Multithread;
use e3_request::{E3Context, E3ContextSnapshot, E3Extension, META_KEY};
use e3_sortition::Sortition;

#[deprecated = "In favour of ThresholdPlaintextAggregatorExtension"]
pub struct PlaintextAggregatorExtension {
    bus: BusHandle,
    sortition: Addr<Sortition>,
}

impl PlaintextAggregatorExtension {
    pub fn create(bus: &BusHandle, sortition: &Addr<Sortition>) -> Box<Self> {
        Box::new(Self {
            bus: bus.clone(),
            sortition: sortition.clone(),
        })
    }
}

const ERROR_PLAINTEXT_FHE_MISSING:&str = "Could not create PlaintextAggregator because the fhe instance it depends on was not set on the context.";
const ERROR_PLAINTEXT_META_MISSING:&str = "Could not create PlaintextAggregator because the meta instance it depends on was not set on the context.";

#[async_trait]
impl E3Extension for PlaintextAggregatorExtension {
    fn on_event(&self, ctx: &mut E3Context, evt: &EnclaveEvent) {
        // Save plaintext aggregator
        let EnclaveEventData::CiphertextOutputPublished(data) = evt.get_data() else {
            return;
        };

        let Some(fhe) = ctx.get_dependency(FHE_KEY) else {
            self.bus.err(
                EnclaveErrorType::PlaintextAggregation,
                anyhow!(ERROR_PLAINTEXT_FHE_MISSING),
            );
            return;
        };

        let Some(ref meta) = ctx.get_dependency(META_KEY) else {
            self.bus.err(
                EnclaveErrorType::PlaintextAggregation,
                anyhow!(ERROR_PLAINTEXT_META_MISSING),
            );
            return;
        };

        let e3_id = data.e3_id.clone();
        let repo = ctx.repositories().plaintext(&e3_id);

        // This is a single ciphertext for the legacy PlaintextAggregator
        let Some(single_ciphertext) = data.ciphertext_output.first() else {
            self.bus.err(
                EnclaveErrorType::PlaintextAggregation,
                anyhow!("Could not extract ciphertext from array"),
            );
            return;
        };

        let sync_state = repo.send(Some(PlaintextAggregatorState::init(
            meta.threshold_m,
            meta.threshold_n,
            meta.seed,
            single_ciphertext.clone(),
        )));

        ctx.set_event_recipient(
            "plaintext",
            Some(
                PlaintextAggregator::new(
                    PlaintextAggregatorParams {
                        fhe: fhe.clone(),
                        bus: self.bus.clone(),
                        sortition: self.sortition.clone(),
                        e3_id: e3_id.clone(),
                    },
                    sync_state,
                )
                .start()
                .into(),
            ),
        );
    }

    async fn hydrate(&self, ctx: &mut E3Context, snapshot: &E3ContextSnapshot) -> Result<()> {
        // No ID on the snapshot -> bail
        if !snapshot.contains("plaintext") {
            return Ok(());
        }

        let repo = ctx.repositories().plaintext(&snapshot.e3_id);
        let sync_state = repo.load().await?;

        // No Snapshot returned from the store -> bail
        if !sync_state.has() {
            return Ok(());
        };

        // Get deps
        let Some(fhe) = ctx.get_dependency(FHE_KEY) else {
            self.bus.err(
                EnclaveErrorType::PlaintextAggregation,
                anyhow!(ERROR_PLAINTEXT_FHE_MISSING),
            );
            return Ok(());
        };

        let value = PlaintextAggregator::new(
            PlaintextAggregatorParams {
                fhe: fhe.clone(),
                bus: self.bus.clone(),
                sortition: self.sortition.clone(),
                e3_id: ctx.e3_id.clone(),
            },
            sync_state,
        )
        .start()
        .into();

        // send to context
        ctx.set_event_recipient("plaintext", Some(value));

        Ok(())
    }
}

pub struct PublicKeyAggregatorExtension {
    bus: BusHandle,
}

impl PublicKeyAggregatorExtension {
    pub fn create(bus: &BusHandle) -> Box<Self> {
        Box::new(Self { bus: bus.clone() })
    }
}

const ERROR_PUBKEY_FHE_MISSING:&str = "Could not create PublicKeyAggregator because the fhe instance it depends on was not set on the context.";
const ERROR_PUBKEY_META_MISSING:&str = "Could not create PublicKeyAggregator because the meta instance it depends on was not set on the context.";

#[async_trait]
impl E3Extension for PublicKeyAggregatorExtension {
    fn on_event(&self, ctx: &mut E3Context, evt: &EnclaveEvent) {
        // Saving the publickey aggregator with deps on E3Requested
        let EnclaveEventData::E3Requested(data) = evt.get_data() else {
            return;
        };

        let Some(fhe) = ctx.get_dependency(FHE_KEY) else {
            self.bus.err(
                EnclaveErrorType::PublickeyAggregation,
                anyhow!(ERROR_PUBKEY_FHE_MISSING),
            );
            return;
        };
        let Some(ref meta) = ctx.get_dependency(META_KEY) else {
            self.bus.err(
                EnclaveErrorType::PublickeyAggregation,
                anyhow!(ERROR_PUBKEY_META_MISSING),
            );
            return;
        };
        let e3_id = data.e3_id.clone();
        let repo = ctx.repositories().publickey(&e3_id);
        let sync_state = repo.send(Some(PublicKeyAggregatorState::init(
            meta.threshold_n,
            meta.seed,
        )));

        let value = create_publickey_aggregator(fhe.clone(), self.bus.clone(), e3_id, sync_state);

        ctx.set_event_recipient("publickey", Some(value));
    }

    async fn hydrate(&self, ctx: &mut E3Context, snapshot: &E3ContextSnapshot) -> Result<()> {
        // No ID on the snapshot -> bail
        if !snapshot.contains("publickey") {
            return Ok(());
        };

        let repo = ctx.repositories().publickey(&ctx.e3_id);
        let sync_state = repo.load().await?;

        // No Snapshot returned from the store -> bail
        if !sync_state.has() {
            return Ok(());
        };

        // Get deps
        let Some(fhe) = ctx.get_dependency(FHE_KEY) else {
            self.bus.err(
                EnclaveErrorType::PublickeyAggregation,
                anyhow!(ERROR_PUBKEY_FHE_MISSING),
            );

            return Ok(());
        };
        let value = create_publickey_aggregator(
            fhe.clone(),
            self.bus.clone(),
            ctx.e3_id.clone(),
            sync_state,
        );

        // send to context
        ctx.set_event_recipient("publickey", Some(value));

        Ok(())
    }
}

fn create_publickey_aggregator(
    fhe: Arc<Fhe>,
    bus: BusHandle,
    e3_id: E3id,
    sync_state: Persistable<PublicKeyAggregatorState>,
) -> Recipient<EnclaveEvent> {
    KeyshareCreatedFilterBuffer::new(
        PublicKeyAggregator::new(PublicKeyAggregatorParams { fhe, bus, e3_id }, sync_state)
            .start()
            .into(),
    )
    .start()
    .into()
}

pub struct ThresholdPlaintextAggregatorExtension {
    bus: BusHandle,
    sortition: Addr<Sortition>,
    multithread: Addr<Multithread>,
}

impl ThresholdPlaintextAggregatorExtension {
    pub fn create(
        bus: &BusHandle,
        sortition: &Addr<Sortition>,
        multithread: &Addr<Multithread>,
    ) -> Box<Self> {
        Box::new(Self {
            bus: bus.clone(),
            sortition: sortition.clone(),
            multithread: multithread.clone(),
        })
    }
}

const ERROR_TRBFV_PLAINTEXT_META_MISSING:&str = "Could not create ThresholdPlaintextAggregator because the meta instance it depends on was not set on the context.";

#[async_trait]
impl E3Extension for ThresholdPlaintextAggregatorExtension {
    fn on_event(&self, ctx: &mut E3Context, evt: &EnclaveEvent) {
        // Save plaintext aggregator
        let EnclaveEventData::CiphertextOutputPublished(data) = evt.get_data() else {
            return;
        };

        let Some(ref meta) = ctx.get_dependency(META_KEY) else {
            self.bus.err(
                EnclaveErrorType::PlaintextAggregation,
                anyhow!(ERROR_TRBFV_PLAINTEXT_META_MISSING),
            );
            return;
        };

        let e3_id = data.e3_id.clone();
        let repo = ctx.repositories().trbfv_plaintext(&e3_id);
        let sync_state = repo.send(Some(ThresholdPlaintextAggregatorState::init(
            meta.threshold_m as u64,
            meta.threshold_n as u64,
            meta.seed,
            data.ciphertext_output.clone(),
            meta.params.clone(),
        )));

        ctx.set_event_recipient(
            "plaintext",
            Some(
                ThresholdPlaintextAggregator::new(
                    ThresholdPlaintextAggregatorParams {
                        bus: self.bus.clone(),
                        sortition: self.sortition.clone(),
                        e3_id: e3_id.clone(),
                        multithread: self.multithread.clone(),
                    },
                    sync_state,
                )
                .start()
                .into(),
            ),
        );
    }

    async fn hydrate(&self, ctx: &mut E3Context, snapshot: &E3ContextSnapshot) -> Result<()> {
        // No ID on the snapshot -> bail
        if !snapshot.contains("plaintext") {
            return Ok(());
        }

        let repo = ctx.repositories().trbfv_plaintext(&snapshot.e3_id);
        let sync_state = repo.load().await?;

        // No Snapshot returned from the store -> bail
        if !sync_state.has() {
            return Ok(());
        };

        let value = ThresholdPlaintextAggregator::new(
            ThresholdPlaintextAggregatorParams {
                bus: self.bus.clone(),
                sortition: self.sortition.clone(),
                e3_id: ctx.e3_id.clone(),
                multithread: self.multithread.clone(),
            },
            sync_state,
        )
        .start()
        .into();

        // send to context
        ctx.set_event_recipient("plaintext", Some(value));

        Ok(())
    }
}
