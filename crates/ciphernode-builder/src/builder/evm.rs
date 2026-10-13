// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! The contract readers and writers of each enabled chain.

use super::{
    restart::work_ended_on_restart,
    startup::{Live, Protocol, Restart, Storage},
    CiphernodeBuilder,
};
use alloy::primitives::Address;
use anyhow::{ensure, Context as _, Result};
use e3_events::{DkgFoldAttestationContext, E3Stage, E3id, EvmEventConfig};
use e3_evm::{
    fetch_randomness_providers, ingestion_heartbeat_files, BondingRegistrySolReader,
    CiphernodeRegistrySol, CiphernodeRegistrySolReader, DataAvailabilityCoordinator,
    DataAvailabilityRepositoryFactory, EnabledChain, EvmChainGatewayHandle, EvmSystemChainBuilder,
    InterfoldSolReader, InterfoldSolWriter, ProviderConfig, RandomnessProviderSolReader,
    SlashingManagerSolReader, SlashingManagerSolWriter, SlashingWriterRepositoryFactory,
};
use e3_sync::NodeRole;
use std::collections::HashMap;
use tracing::{error, info};

impl CiphernodeBuilder {
    /// Attach the data-availability coordinator and the configured contract readers and writers of
    /// each enabled chain.
    pub(super) async fn setup_evm_system(
        &self,
        storage: &Storage,
        restart: &Restart,
        live: &mut Live,
        protocol: &Protocol,
    ) -> Result<(EvmEventConfig, Vec<EvmChainGatewayHandle>)> {
        let Restart {
            kept_failures,
            committee_finalizer,
            lifecycle_stages,
            dkg_fold_contexts_by_e3,
        } = restart;
        let Protocol {
            canonical_keys,
            terminal_plaintext_e3s,
            selected_party_ids,
            slashing_managers,
            ..
        } = protocol;
        let Live {
            bus,
            provider_cache,
            ..
        } = live;
        let (chains, repositories, eventstore) = (
            &storage.chains,
            &storage.repositories,
            &storage.seq_eventstore,
        );
        let full = self.role() == NodeRole::Full;
        let progress = self
            .ingestion_heartbeat_dir
            .clone()
            .map(ingestion_heartbeat_files)
            .transpose()
            .context("ingestion heartbeat directory")?;
        let mut evm_config = EvmEventConfig::new();
        let mut gateways = Vec::new();
        let finished = work_ended_on_restart(lifecycle_stages, kept_failures);
        for (enabled, slashing_manager) in chains
            .iter()
            .zip(slashing_managers.iter().copied())
            .filter_map(|(chain, slashing_manager)| Some((chain.as_enabled()?, slashing_manager)))
        {
            let EnabledChain {
                config: chain,
                chain_id,
                provider,
            } = enabled;
            let chain_id = *chain_id;
            if full && chain.data_availability.is_none() {
                anyhow::bail!(
                    "chain '{}' has Interfold enabled but no data_availability reader; this protocol release must retrieve proof-backed ciphertext outputs",
                    chain.name
                );
            }
            DataAvailabilityCoordinator::attach(
                bus,
                chain_id,
                chain.data_availability.as_ref(),
                repositories.data_availability_recovery(chain_id),
                &finished,
            )
            .await?;
            if full {
                validate_vrf_chain_id(chain_id)?;
            }
            // Delay ingestion until the configured number of confirmations is present.
            let ingestion_confirmations = chain.ingestion_confirmations()?;
            evm_config.insert(chain_id, chain.try_into()?);

            let provider_factory = ProviderConfig::for_chain(chain)?.into_read_provider_factory();

            let max_log_window = chain.rpc_log_range_blocks()?;
            let mut system = EvmSystemChainBuilder::new(bus, provider);
            system
                .with_provider_factory(provider_factory.clone())
                .with_buffer_limit(self.max_buffered_evm_events)
                .with_max_log_window(max_log_window);
            if let Some(progress) = &progress {
                system.with_progress_sink(progress.clone());
            }

            let interfold = &chain.contracts.interfold;
            // A bootstrap node reads only the Interfold contract.
            if !full {
                system.with_contract(interfold.address()?, move |next| {
                    InterfoldSolReader::setup(&next).recipient()
                });
                gateways.push(system.build_with_readiness()?);
                continue;
            }

            // Interfold reader and writer
            let write_provider = provider_cache.ensure_write_provider(chain).await?;
            let chain_failure_stages = on_chain(lifecycle_stages, chain_id)
                .filter(|(_, stage)| {
                    matches!(
                        stage,
                        E3Stage::Requested
                            | E3Stage::CommitteeFinalized
                            | E3Stage::KeyPublished
                            | E3Stage::CiphertextReady
                    )
                })
                .map(|(e3_id, stage)| (e3_id.clone(), stage.clone()))
                .collect();
            let chain_failure_settlements = on_chain(lifecycle_stages, chain_id)
                .filter(|(_, stage)| **stage == E3Stage::Failed)
                .map(|(e3_id, _)| e3_id.clone())
                .collect();
            InterfoldSolWriter::attach_with_recovery(
                bus,
                write_provider.clone(),
                interfold.address()?,
                on_chain(selected_party_ids, chain_id)
                    .map(|(e3_id, party_id)| (e3_id.clone(), *party_id))
                    .collect(),
                request_registries(dkg_fold_contexts_by_e3, chain_id),
                chain_failure_stages,
                chain_failure_settlements,
                terminal_plaintext_e3s
                    .iter()
                    .filter(|id| id.chain_id() == chain_id)
                    .cloned()
                    .collect(),
                canonical_keys.clone(),
                eventstore.clone(),
            );
            system.with_contract(interfold.address()?, move |next| {
                InterfoldSolReader::setup(&next).recipient()
            });

            // BondingRegistry reader
            system.with_contract(chain.contracts.bonding_registry.address()?, move |next| {
                BondingRegistrySolReader::setup(&next).recipient()
            });

            // CiphernodeRegistry and randomness provider readers, and the registry writer
            let registry = &chain.contracts.ciphernode_registry;
            let registry_address = registry.address()?;
            let registry_provider = provider.clone();
            let registry_provider_factory = provider_factory.clone();
            system.with_contract(registry_address, move |next| {
                CiphernodeRegistrySolReader::setup_with_factory(
                    &next,
                    registry_provider,
                    Some(registry_provider_factory),
                    ingestion_confirmations,
                )
                .recipient()
            });

            let randomness_addresses = fetch_randomness_providers(
                provider.provider(),
                registry_address,
                registry.deploy_block().unwrap_or(0),
                max_log_window,
            )
            .await?;
            if randomness_addresses.is_empty() {
                return Err(anyhow::anyhow!(
                    "ciphernode registry {} has no randomness provider configured",
                    registry_address
                ));
            }
            for randomness_address in randomness_addresses {
                let randomness_read_provider = provider.clone();
                let randomness_provider_factory = provider_factory.clone();
                system.with_contract(randomness_address, move |next| {
                    RandomnessProviderSolReader::setup_with_factory(
                        &next,
                        randomness_read_provider,
                        Some(randomness_provider_factory),
                        registry_address,
                    )
                    .recipient()
                });
            }

            // TODO: Should we not let this pass and just use '?'?
            // Above if we include interfold in the config and we don't have a wallet it will fail
            match provider_cache.ensure_write_provider(chain).await {
                Ok(write_provider) => {
                    CiphernodeRegistrySol::attach_writer_with_recovery(
                        bus,
                        write_provider.clone(),
                        registry_address,
                        request_registries(dkg_fold_contexts_by_e3, chain_id),
                        on_chain(&committee_finalizer.tickets, chain_id)
                            .map(|(e3_id, ticket)| (e3_id.clone(), ticket.clone()))
                            .collect(),
                    );
                    info!("CiphernodeRegistrySolWriter attached for publishing committees");
                }
                Err(e) => error!(
                    "Failed to create write provider (likely no wallet configured), skipping writer attachment: {}",
                    e
                ),
            }

            // SlashingManager reader and writer
            let slashing_manager = slashing_manager.ok_or_else(|| {
                anyhow::anyhow!(
                    "Slashing manager is enabled but no contract address configured for chain {}",
                    chain.name
                )
            })?;
            system.with_contract(slashing_manager, move |next| {
                SlashingManagerSolReader::setup(&next).recipient()
            });
            match provider_cache.ensure_write_provider(chain).await {
                Ok(write_provider) => {
                    match SlashingManagerSolWriter::attach_with_recovery(
                        bus,
                        write_provider.clone(),
                        slashing_manager,
                        repositories.slashing_writer_recovery(chain_id),
                    )
                    .await
                    {
                        Ok(_) => {
                            info!("SlashingManagerSolWriter attached for fault submission");
                        }
                        Err(e) => {
                            error!("Failed to attach SlashingManagerSolWriter, skipping: {}", e)
                        }
                    }
                }
                Err(e) => error!(
                    "Failed to create write provider for SlashingManager, skipping: {}",
                    e
                ),
            }

            gateways.push(system.build_with_readiness()?);
        }

        Ok((evm_config, gateways))
    }
}

/// The entries of `by_e3` whose E3 belongs to `chain_id`.
fn on_chain<V>(by_e3: &HashMap<E3id, V>, chain_id: u64) -> impl Iterator<Item = (&E3id, &V)> {
    by_e3
        .iter()
        .filter(move |(e3_id, _)| e3_id.chain_id() == chain_id)
}

/// The request-time CiphernodeRegistry of each restored E3 of `chain_id`.
fn request_registries(
    dkg_fold_contexts_by_e3: &HashMap<E3id, DkgFoldAttestationContext>,
    chain_id: u64,
) -> HashMap<E3id, Address> {
    on_chain(dkg_fold_contexts_by_e3, chain_id)
        .map(|(e3_id, context)| (e3_id.clone(), context.registry))
        .collect()
}

fn validate_vrf_chain_id(chain_id: u64) -> Result<()> {
    ensure!(
        matches!(chain_id, 1 | 1_337 | 31_337 | 11_155_111),
        "VRF sortition supports Ethereum mainnet, Sepolia, and local development chains only; received chain_id {chain_id}"
    );
    Ok(())
}

pub(super) async fn wait_for_evm_gateways(gateways: Vec<EvmChainGatewayHandle>) -> Result<()> {
    futures::future::try_join_all(
        gateways
            .into_iter()
            .map(EvmChainGatewayHandle::wait_until_live),
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_vrf_chain_id;

    #[test]
    fn supports_ethereum_vrf_chains() {
        for chain_id in [1, 11_155_111, 31_337, 1_337] {
            assert!(validate_vrf_chain_id(chain_id).is_ok());
        }
    }

    #[test]
    fn rejects_arbitrum_vrf_chains() {
        let error = validate_vrf_chain_id(42_161).expect_err("Arbitrum must be rejected");

        assert!(error.to_string().contains("Ethereum mainnet"));
    }
}
