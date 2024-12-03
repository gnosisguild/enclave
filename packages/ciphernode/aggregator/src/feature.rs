use actix::{Actor,Addr};
use enclave_core::{EnclaveErrorType, EnclaveEvent, EventBus};
use router::{E3Feature, E3RequestContext, E3RequestContextSnapshot, RepositoriesFactory};
use sortition::Sortition;
use data::{FromSnapshotWithParams, Snapshot};
use crate::{PlaintextAggregator, PlaintextAggregatorParams, PlaintextAggregatorState, PublicKeyAggregator, PublicKeyAggregatorParams, PublicKeyAggregatorState};


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

        let _ = ctx.set_event_recipient(
            "plaintext",
            Some(
                PlaintextAggregator::new(
                    PlaintextAggregatorParams {
                        fhe: fhe.clone(),
                        bus: self.bus.clone(),
                        store: ctx.repositories().plaintext(&e3_id),
                        sortition: self.sortition.clone(),
                        e3_id,
                        src_chain_id: meta.src_chain_id,
                    },
                    PlaintextAggregatorState::init(
                        meta.threshold_m,
                        meta.seed,
                        data.ciphertext_output.clone(),
                    ),
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

        let store = ctx.repositories().plaintext(&snapshot.e3_id);

        // No Snapshot returned from the store -> bail
        let Some(snap) = store.read().await? else {
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

        let value = PlaintextAggregator::from_snapshot(
            PlaintextAggregatorParams {
                fhe: fhe.clone(),
                bus: self.bus.clone(),
                store,
                sortition: self.sortition.clone(),
                e3_id: ctx.e3_id.clone(),
                src_chain_id: meta.src_chain_id,
            },
            snap,
        )
        .await?
        .start();

        // send to context
        ctx.set_event_recipient("plaintext", Some(value.into()));

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

        let _ = ctx.set_event_recipient(
            "publickey",
            Some(
                PublicKeyAggregator::new(
                    PublicKeyAggregatorParams {
                        fhe: fhe.clone(),
                        bus: self.bus.clone(),
                        store: ctx.repositories().publickey(&e3_id),
                        sortition: self.sortition.clone(),
                        e3_id,
                        src_chain_id: meta.src_chain_id,
                    },
                    PublicKeyAggregatorState::init(meta.threshold_m, meta.seed),
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

        let repository = ctx.repositories().publickey(&ctx.e3_id);

        // No Snapshot returned from the store -> bail
        let Some(snap) = repository.read().await? else {
            return Ok(());
        };

        // Get deps
        let Some(fhe) = ctx.fhe.clone() else {
            self.bus.err(
                EnclaveErrorType::PublickeyAggregation,
                anyhow!(ERROR_PUBKEY_FHE_MISSING),
            );

            return Ok(());
        };

        let Some(meta) = ctx.meta.clone() else {
            self.bus.err(
                EnclaveErrorType::PublickeyAggregation,
                anyhow!(ERROR_PUBKEY_META_MISSING),
            );

            return Ok(());
        };

        let value = PublicKeyAggregator::from_snapshot(
            PublicKeyAggregatorParams {
                fhe: fhe.clone(),
                bus: self.bus.clone(),
                store: repository,
                sortition: self.sortition.clone(),
                e3_id: ctx.e3_id.clone(),
                src_chain_id: meta.src_chain_id,
            },
            snap,
        )
        .await?
        .start();

        // send to context
        ctx.set_event_recipient("publickey",Some(value.into()));

        Ok(())
    }
}
