// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Per-E3 forward-secrecy cipher management.
//!
//! Each E3 round gets its own randomly-generated AES-256-GCM key ("E3 key"). The E3 key is
//! persisted in the KV store **encrypted under the node's master cipher**, so a restart can
//! re-derive the per-E3 cipher without user interaction.
//!
//! When the E3 round reaches a terminal state (`E3RequestComplete`) the E3 key is deleted from
//! the store.  From that point on, `SensitiveBytes` values encrypted with the per-E3 cipher are
//! permanently irrecoverable, even if the master passphrase is later leaked
//! (forward-secrecy).

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use e3_crypto::Cipher;
use e3_data::{Repositories, RepositoriesFactory, Repository};
use e3_events::{
    E3Failed, E3Stage, E3id, EnclaveEvent, EnclaveEventData, Event, EventConstructorWithTimestamp,
    EventSource, FailureReason, StoreKeys, Unsequenced,
};
use std::sync::Arc;
use tracing::{debug, info};

use crate::{E3Context, E3ContextSnapshot, E3Extension, TypedKey};

/// Context dependency key under which the per-E3 `Cipher` is stored.
pub const E3_CIPHER_KEY: TypedKey<Arc<Cipher>> = TypedKey::new("e3_cipher");

// ── Repository helpers ────────────────────────────────────────────────────────

pub trait E3CipherRepositoryFactory {
    fn e3_cipher(&self, e3_id: &E3id) -> Repository<Vec<u8>>;
}

impl E3CipherRepositoryFactory for Repositories {
    fn e3_cipher(&self, e3_id: &E3id) -> Repository<Vec<u8>> {
        // `base` (absolute), not `scope` (relative): the per-E3 key must live at a fixed,
        // node-global location so that the shared `Multithread` actor — which holds the root
        // store, not this per-E3 context scope — resolves the exact same key. The e3_id is
        // already in the key, so it stays unique per round.
        Repository::new(self.store.base(StoreKeys::e3_key(e3_id)))
    }
}

// ── Extension ────────────────────────────────────────────────────────────────

/// An [`E3Extension`] that injects a per-E3 forward-secrecy cipher into the context.
///
/// Register this extension **before** any extension that consumes `E3_CIPHER_KEY` (e.g.
/// `ThresholdKeyshareExtension`) so that the cipher is present by the time it is needed.
pub struct E3CipherExtension {
    master_cipher: Arc<Cipher>,
}

impl E3CipherExtension {
    pub fn create(master_cipher: &Arc<Cipher>) -> Box<Self> {
        Box::new(Self {
            master_cipher: master_cipher.clone(),
        })
    }

    /// Generate a new E3 key, persist it (encrypted under `master`) and return a `Cipher` for it.
    fn create_and_store(&self, repo: &Repository<Vec<u8>>, e3_id: &E3id) -> Result<Arc<Cipher>> {
        let e3_cipher = Cipher::generate()?;
        // Encrypt the raw key under the master cipher before storing.
        let mut key_copy: Vec<u8> = e3_cipher.key_bytes().as_slice().to_vec();
        let encrypted = self.master_cipher.encrypt_data(&mut key_copy)?;
        repo.write(&encrypted);
        debug!(e3_id = %e3_id, "generated and stored new E3 cipher key");
        Ok(Arc::new(e3_cipher))
    }

    /// Load an existing E3 key from the store and return the corresponding `Cipher`.
    async fn load(&self, repo: &Repository<Vec<u8>>, e3_id: &E3id) -> Result<Option<Arc<Cipher>>> {
        let Some(encrypted_key) = repo.read().await? else {
            return Ok(None);
        };
        let raw = self.master_cipher.decrypt_data(&encrypted_key)?;
        let cipher = Cipher::from_key_bytes(raw)?;
        debug!(e3_id = %e3_id, "loaded existing E3 cipher key");
        Ok(Some(Arc::new(cipher)))
    }

    /// Delete the E3 key from the store — called when the round completes.
    fn purge(repo: &Repository<Vec<u8>>, e3_id: &E3id) {
        repo.clear();
        info!(e3_id = %e3_id, "purged E3 cipher key (forward secrecy)");
    }
}

#[async_trait]
impl E3Extension for E3CipherExtension {
    fn on_event(&self, ctx: &mut E3Context, evt: &EnclaveEvent) {
        match evt.get_data() {
            // Create the E3 cipher the moment this round is first seen.
            EnclaveEventData::E3Requested(data) => {
                if ctx.get_dependency(E3_CIPHER_KEY).is_some() {
                    return;
                }
                let repo = ctx.repositories().e3_cipher(&data.e3_id);
                match self.create_and_store(&repo, &data.e3_id) {
                    Ok(cipher) => ctx.set_dependency(E3_CIPHER_KEY, cipher),
                    Err(e) => {
                        tracing::error!(
                            e3_id = %data.e3_id,
                            "failed to create E3 cipher: {e}; aborting round to preserve forward secrecy"
                        );
                        let fail_evt = EnclaveEvent::<Unsequenced>::new_with_timestamp(
                            EnclaveEventData::from(E3Failed {
                                e3_id: data.e3_id.clone(),
                                failed_at_stage: E3Stage::Requested,
                                reason: FailureReason::None,
                            }),
                            None,
                            0,
                            None,
                            EventSource::Local,
                        )
                        .into_sequenced(0);
                        ctx.forward_message_now(&fail_evt);
                    }
                }
            }
            // Purge the E3 key on completion.
            EnclaveEventData::E3RequestComplete(data) => {
                let repo = ctx.repositories().e3_cipher(&data.e3_id);
                Self::purge(&repo, &data.e3_id);
            }
            _ => {}
        }
    }

    async fn hydrate(&self, ctx: &mut E3Context, snapshot: &E3ContextSnapshot) -> Result<()> {
        if !snapshot.contains("e3_cipher") {
            return Ok(());
        }
        let repo = ctx.repositories().e3_cipher(&snapshot.e3_id);
        match self.load(&repo, &snapshot.e3_id).await? {
            Some(cipher) => ctx.set_dependency(E3_CIPHER_KEY, cipher),
            None => {
                return Err(anyhow!(
                    "E3 cipher key for {} not found in store during hydration; \
                     the round may have completed before the node restarted",
                    snapshot.e3_id
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix::Actor;
    use e3_data::{DataStore, InMemStore, Repositories, Repository};
    use e3_events::{E3id, EnclaveEvent};

    fn master() -> Arc<Cipher> {
        // Synchronous construction via from_key_bytes so tests don't need async for the master.
        Arc::new(Cipher::from_key_bytes(vec![0xABu8; 32]).unwrap())
    }

    fn test_repos() -> Repositories {
        let store = InMemStore::new(false).start();
        DataStore::from_in_mem(&store).into()
    }

    fn e3id() -> E3id {
        E3id::new("1", 1)
    }

    fn repo_for(repos: &Repositories, id: &E3id) -> Repository<Vec<u8>> {
        repos.e3_cipher(id)
    }

    // ── create_and_store / load round-trip ───────────────────────────────────

    #[actix::test]
    async fn create_store_load_round_trips() {
        let ext = E3CipherExtension::create(&master());
        let repos = test_repos();
        let id = e3id();
        let repo = repo_for(&repos, &id);

        let cipher = ext.create_and_store(&repo, &id).unwrap();

        // Load should recover the same key material.
        let loaded = ext.load(&repo, &id).await.unwrap().unwrap();
        assert_eq!(
            cipher.key_bytes().as_slice(),
            loaded.key_bytes().as_slice(),
            "loaded key must match the one that was stored"
        );
    }

    // Regression: the per-E3 key must resolve to the SAME absolute location whether accessed
    // through a per-E3 context-scoped store (as `E3CipherExtension` does) or through the root
    // store (as the shared `Multithread` actor does). A relative `scope` here would write under
    // the context scope and the root reader would miss it — surfacing as "Could not decrypt data"
    // during key generation.
    #[actix::test]
    async fn e3_cipher_key_resolves_at_same_location_across_scopes() {
        use e3_data::RepositoriesFactory;

        let ext = E3CipherExtension::create(&master());
        let root = test_repos();
        let id = e3id();

        // Writer side: a deeply scoped store, mimicking the router's per-E3 context scope.
        let scoped_repos: Repositories = root
            .store
            .scope(StoreKeys::router())
            .scope(StoreKeys::context(&id))
            .repositories();
        let stored = ext
            .create_and_store(&scoped_repos.e3_cipher(&id), &id)
            .unwrap();

        // Reader side: the root store (what Multithread holds) must find the same key.
        let loaded = ext
            .load(&root.e3_cipher(&id), &id)
            .await
            .unwrap()
            .expect("root reader must resolve the key written through the scoped store");
        assert_eq!(
            stored.key_bytes().as_slice(),
            loaded.key_bytes().as_slice(),
            "key must be identical regardless of the store scope it is accessed through"
        );
    }

    #[actix::test]
    async fn load_returns_none_when_no_key_stored() {
        let ext = E3CipherExtension::create(&master());
        let repos = test_repos();
        let id = e3id();
        let repo = repo_for(&repos, &id);

        let result = ext.load(&repo, &id).await.unwrap();
        assert!(result.is_none());
    }

    // ── purge ────────────────────────────────────────────────────────────────

    #[actix::test]
    async fn purge_removes_key_from_store() {
        let ext = E3CipherExtension::create(&master());
        let repos = test_repos();
        let id = e3id();
        let repo = repo_for(&repos, &id);

        ext.create_and_store(&repo, &id).unwrap();
        assert!(ext.load(&repo, &id).await.unwrap().is_some());

        E3CipherExtension::purge(&repo, &id);
        assert!(
            ext.load(&repo, &id).await.unwrap().is_none(),
            "key must be absent after purge"
        );
    }

    // ── wrong master cipher ──────────────────────────────────────────────────

    #[actix::test]
    async fn load_fails_with_wrong_master() {
        let ext_a = E3CipherExtension::create(&master());
        let wrong_master = Arc::new(Cipher::from_key_bytes(vec![0x01u8; 32]).unwrap());
        let ext_b = E3CipherExtension::create(&wrong_master);

        let repos = test_repos();
        let id = e3id();
        let repo = repo_for(&repos, &id);

        ext_a.create_and_store(&repo, &id).unwrap();
        // Decryption with a different master must fail.
        assert!(ext_b.load(&repo, &id).await.is_err());
    }

    // ── on_event ─────────────────────────────────────────────────────────────

    fn make_context(repos: Repositories, id: E3id) -> E3Context {
        use crate::HetrogenousMap;
        use e3_data::Repository;

        E3Context {
            e3_id: id,
            repository: Repository::new(repos.store.clone()),
            recipients: std::collections::HashMap::new(),
            dependencies: HetrogenousMap::new(),
        }
    }

    fn e3_requested_event(id: E3id) -> EnclaveEvent {
        use e3_events::{E3Requested, Sequenced};
        EnclaveEvent::<Sequenced>::test_event("e3_requested")
            .data(E3Requested {
                e3_id: id,
                ..E3Requested::default()
            })
            .seq(1)
            .build()
    }

    fn e3_complete_event(id: E3id) -> EnclaveEvent {
        use e3_events::{E3RequestComplete, Sequenced};
        EnclaveEvent::<Sequenced>::test_event("e3_complete")
            .data(E3RequestComplete { e3_id: id })
            .seq(2)
            .build()
    }

    #[actix::test]
    async fn on_event_e3_requested_sets_cipher_in_context() {
        let ext = E3CipherExtension::create(&master());
        let repos = test_repos();
        let id = e3id();
        let mut ctx = make_context(repos, id.clone());

        let evt = e3_requested_event(id.clone());
        ext.on_event(&mut ctx, &evt);

        assert!(
            ctx.get_dependency(E3_CIPHER_KEY).is_some(),
            "E3_CIPHER_KEY must be set in context after E3Requested"
        );
    }

    #[actix::test]
    async fn on_event_e3_requested_is_idempotent() {
        let ext = E3CipherExtension::create(&master());
        let repos = test_repos();
        let id = e3id();
        let mut ctx = make_context(repos, id.clone());

        let evt = e3_requested_event(id.clone());
        ext.on_event(&mut ctx, &evt);
        let key_first = ctx
            .get_dependency(E3_CIPHER_KEY)
            .unwrap()
            .key_bytes()
            .clone();

        // Second call must not overwrite the key.
        ext.on_event(&mut ctx, &evt);
        let key_second = ctx
            .get_dependency(E3_CIPHER_KEY)
            .unwrap()
            .key_bytes()
            .clone();

        assert_eq!(
            key_first.as_slice(),
            key_second.as_slice(),
            "repeated E3Requested must not regenerate the key"
        );
    }

    #[actix::test]
    async fn on_event_e3_request_complete_purges_key_from_store() {
        let ext = E3CipherExtension::create(&master());
        let repos = test_repos();
        let id = e3id();
        let mut ctx = make_context(repos.clone(), id.clone());

        // Seed the context with a key.
        let evt_req = e3_requested_event(id.clone());
        ext.on_event(&mut ctx, &evt_req);

        // Confirm it's in the store.
        let repo = repo_for(&repos, &id);
        assert!(ext.load(&repo, &id).await.unwrap().is_some());

        // Complete the round.
        let evt_done = e3_complete_event(id.clone());
        ext.on_event(&mut ctx, &evt_done);

        // Key must be gone.
        assert!(
            ext.load(&repo, &id).await.unwrap().is_none(),
            "E3 key must be purged after E3RequestComplete"
        );
    }

    // ── hydrate ──────────────────────────────────────────────────────────────

    #[actix::test]
    async fn hydrate_restores_cipher_from_store() {
        let ext = E3CipherExtension::create(&master());
        let repos = test_repos();
        let id = e3id();

        // Pre-store a key as if a previous `on_event(E3Requested)` ran.
        let repo = repo_for(&repos, &id);
        let original = ext.create_and_store(&repo, &id).unwrap();

        let mut ctx = make_context(repos, id.clone());
        let snapshot = E3ContextSnapshot {
            e3_id: id.clone(),
            recipients: vec![],
            dependencies: vec!["e3_cipher".to_string()],
        };

        ext.hydrate(&mut ctx, &snapshot).await.unwrap();

        let restored = ctx.get_dependency(E3_CIPHER_KEY).unwrap();
        assert_eq!(
            original.key_bytes().as_slice(),
            restored.key_bytes().as_slice(),
            "hydrated cipher must match the stored key"
        );
    }

    #[actix::test]
    async fn hydrate_skips_when_snapshot_has_no_e3_cipher() {
        let ext = E3CipherExtension::create(&master());
        let repos = test_repos();
        let id = e3id();
        let mut ctx = make_context(repos, id.clone());

        let snapshot = E3ContextSnapshot {
            e3_id: id.clone(),
            recipients: vec![],
            dependencies: vec![], // no "e3_cipher"
        };

        ext.hydrate(&mut ctx, &snapshot).await.unwrap();
        assert!(ctx.get_dependency(E3_CIPHER_KEY).is_none());
    }

    #[actix::test]
    async fn hydrate_errors_when_key_missing_from_store() {
        let ext = E3CipherExtension::create(&master());
        let repos = test_repos();
        let id = e3id();
        let mut ctx = make_context(repos, id.clone());

        // Snapshot claims the cipher was present, but nothing is in the store.
        let snapshot = E3ContextSnapshot {
            e3_id: id.clone(),
            recipients: vec![],
            dependencies: vec!["e3_cipher".to_string()],
        };

        let result = ext.hydrate(&mut ctx, &snapshot).await;
        assert!(
            result.is_err(),
            "hydrate must return Err when key is missing from store"
        );
    }
}
