use crate::{
    PlaintextAggregator, PlaintextAggregatorParams, PlaintextAggregatorState,
    PlaintextRepositoryFactory, PublicKeyAggregator, PublicKeyAggregatorParams,
    PublicKeyAggregatorState, PublicKeyRepositoryFactory,
};
use actix::{Actor, Addr};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use data::{AutoPersist, RepositoriesFactory};
use enclave_core::{BusError, EnclaveErrorType, EnclaveEvent, EventBus};
use router::{E3Feature, E3RequestContext, E3RequestContextSnapshot};
use sortition::Sortition;

pub struct PlaintextAggregatorFeature {
    bus: Addr<EventBus>,
    sortition: Addr<Sortition>,
}
impl PlaintextAggregatorFeature {
    pub fn create(bus: &Addr<EventBus>, sortition: &Addr<Sortition>) -> Box<Self> {
        Box::new(Self {
            bus: bus.clone(),
            sortition: sortition.clone(),
        })
    }
}

const ERROR_PLAINTEXT_FHE_MISSING:&str = "Could not create PlaintextAggregator because the fhe instance it depends on was not set on the context.";
const ERROR_PLAINTEXT_META_MISSING:&str = "Could not create PlaintextAggregator because the meta instance it depends on was not set on the context.";

#[async_trait]
impl E3Feature for PlaintextAggregatorFeature {
    fn on_event(&self, ctx: &mut E3RequestContext, evt: &EnclaveEvent) {
        // Save plaintext aggregator
        let EnclaveEvent::CiphertextOutputPublished { data, .. } = evt else {
            return;
        };

        let Some(fhe) = ctx.get_fhe() else {
            self.bus.err(
                EnclaveErrorType::PlaintextAggregation,
                anyhow!(ERROR_PLAINTEXT_FHE_MISSING),
            );
            return;
        };

        let Some(ref meta) = ctx.get_meta() else {
            self.bus.err(
                EnclaveErrorType::PlaintextAggregation,
                anyhow!(ERROR_PLAINTEXT_META_MISSING),
            );
            return;
        };

        let e3_id = data.e3_id.clone();
        let repo = ctx.repositories().plaintext(&e3_id);
        let sync_state = repo.send(Some(PlaintextAggregatorState::init(
            meta.threshold_m,
            meta.seed,
            data.ciphertext_output.clone(),
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
                        src_chain_id: meta.src_chain_id,
                    },
                    sync_state,
                )
                .start()
                .into(),
            ),
        );
    }

    async fn hydrate(
        &self,
        ctx: &mut E3RequestContext,
        snapshot: &E3RequestContextSnapshot,
    ) -> Result<()> {
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
        let Some(fhe) = ctx.get_fhe() else {
            self.bus.err(
                EnclaveErrorType::PlaintextAggregation,
                anyhow!(ERROR_PLAINTEXT_FHE_MISSING),
            );
            return Ok(());
        };

        let Some(ref meta) = ctx.get_meta() else {
            self.bus.err(
                EnclaveErrorType::PlaintextAggregation,
                anyhow!(ERROR_PLAINTEXT_META_MISSING),
            );
            return Ok(());
        };

        let value = PlaintextAggregator::new(
            PlaintextAggregatorParams {
                fhe: fhe.clone(),
                bus: self.bus.clone(),
                sortition: self.sortition.clone(),
                e3_id: ctx.e3_id.clone(),
                src_chain_id: meta.src_chain_id,
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

pub struct PublicKeyAggregatorFeature {
    bus: Addr<EventBus>,
    sortition: Addr<Sortition>,
}

impl PublicKeyAggregatorFeature {
    pub fn create(bus: &Addr<EventBus>, sortition: &Addr<Sortition>) -> Box<Self> {
        Box::new(Self {
            bus: bus.clone(),
            sortition: sortition.clone(),
        })
    }
}

const ERROR_PUBKEY_FHE_MISSING:&str = "Could not create PublicKeyAggregator because the fhe instance it depends on was not set on the context.";
const ERROR_PUBKEY_META_MISSING:&str = "Could not create PublicKeyAggregator because the meta instance it depends on was not set on the context.";

#[async_trait]
impl E3Feature for PublicKeyAggregatorFeature {
    fn on_event(&self, ctx: &mut E3RequestContext, evt: &EnclaveEvent) {
        // Saving the publickey aggregator with deps on E3Requested
        let EnclaveEvent::E3Requested { data, .. } = evt else {
            return;
        };

        let Some(fhe) = ctx.get_fhe() else {
            self.bus.err(
                EnclaveErrorType::PublickeyAggregation,
                anyhow!(ERROR_PUBKEY_FHE_MISSING),
            );
            return;
        };
        let Some(ref meta) = ctx.get_meta() else {
            self.bus.err(
                EnclaveErrorType::PublickeyAggregation,
                anyhow!(ERROR_PUBKEY_META_MISSING),
            );
            return;
        };

        let e3_id = data.e3_id.clone();
        let repo = ctx.repositories().publickey(&e3_id);
        let sync_state = repo.send(Some(PublicKeyAggregatorState::init(
            meta.threshold_m,
            meta.seed,
        )));
        ctx.set_event_recipient(
            "publickey",
            Some(
                PublicKeyAggregator::new(
                    PublicKeyAggregatorParams {
                        fhe: fhe.clone(),
                        bus: self.bus.clone(),
                        sortition: self.sortition.clone(),
                        e3_id,
                        src_chain_id: meta.src_chain_id,
                    },
                    sync_state,
                )
                .start()
                .into(),
            ),
        );
    }

    async fn hydrate(
        &self,
        ctx: &mut E3RequestContext,
        snapshot: &E3RequestContextSnapshot,
    ) -> Result<()> {
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
        let Some(fhe) = ctx.get_fhe() else {
            self.bus.err(
                EnclaveErrorType::PublickeyAggregation,
                anyhow!(ERROR_PUBKEY_FHE_MISSING),
            );

            return Ok(());
        };

        let Some(meta) = ctx.get_meta() else {
            self.bus.err(
                EnclaveErrorType::PublickeyAggregation,
                anyhow!(ERROR_PUBKEY_META_MISSING),
            );

            return Ok(());
        };

        let value = PublicKeyAggregator::new(
            PublicKeyAggregatorParams {
                fhe: fhe.clone(),
                bus: self.bus.clone(),
                sortition: self.sortition.clone(),
                e3_id: ctx.e3_id.clone(),
                src_chain_id: meta.src_chain_id,
            },
            sync_state,
        )
        .start()
        .into();

        // send to context
        ctx.set_event_recipient("publickey", Some(value));

        Ok(())
    }
}
