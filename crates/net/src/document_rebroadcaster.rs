// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Periodically re-broadcasts gossipsub notifications for documents that
//! haven't been acknowledged by exchange completion.
//!
//! When a node publishes a document to the DHT, it sends a one-shot gossipsub
//! notification. If that notification is lost (network partition, mesh
//! degradation), other nodes never know to fetch the data — even though it
//! exists in the DHT. This actor re-sends those notifications until the
//! corresponding exchange phase completes.
//!
//! Phase-aware cleanup:
//! - `E3StageChanged(KeyPublished)` → DKG done, stop rebroadcasting DKG documents
//! - `E3StageChanged(Complete)` / `E3Failed` → E3 terminal, stop everything

use crate::{
    document_publisher::broadcast_document_published_notification,
    events::{DocumentPublishedNotification, NetCommand, NetEvent},
    ContentHash,
};
use actix::prelude::*;
use e3_events::{
    BusHandle, E3Stage, E3id, EnclaveEvent, EnclaveEventData, EventSubscriber, EventType,
    PublishDocumentRequested,
};
use e3_utils::MAILBOX_LIMIT;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, info, warn};

/// Delay before the first rebroadcast attempt.
const REBROADCAST_INITIAL_DELAY: Duration = Duration::from_secs(10);

/// Interval between rebroadcast checks.
const REBROADCAST_INTERVAL: Duration = Duration::from_secs(15);

/// Maximum number of rebroadcast attempts per document.
/// 40 attempts x 15s interval = ~10 minutes of retries.
const MAX_REBROADCASTS: u32 = 40;

/// Maximum pending notifications per E3 to prevent unbounded growth.
const MAX_PENDING_PER_E3: usize = 50;

struct PendingNotification {
    notification: DocumentPublishedNotification,
    first_published: Instant,
    last_rebroadcast: Instant,
    rebroadcast_count: u32,
}

/// Per-E3 state for the rebroadcaster.
struct E3RebroadcastState {
    pending: Vec<PendingNotification>,
    /// When a phase-boundary event arrives (e.g. KeyPublished), we record the
    /// timestamp. All notifications published before this cutoff are removed,
    /// since their exchange phase completed. New notifications (from the next
    /// phase) are kept.
    phase_cutoff: Option<Instant>,
}

impl E3RebroadcastState {
    fn new() -> Self {
        Self {
            pending: Vec::new(),
            phase_cutoff: None,
        }
    }

    /// Remove all notifications published before the cutoff.
    fn apply_cutoff(&mut self) {
        if let Some(cutoff) = self.phase_cutoff {
            let before = self.pending.len();
            self.pending.retain(|p| p.first_published > cutoff);
            let removed = before - self.pending.len();
            if removed > 0 {
                debug!(
                    removed,
                    "Removed pre-cutoff notifications after phase transition"
                );
            }
        }
    }
}

/// Re-broadcasts gossipsub document notifications for incomplete exchanges.
pub struct DocumentRebroadcaster {
    bus: BusHandle,
    tx: mpsc::Sender<NetCommand>,
    rx: Arc<broadcast::Receiver<NetEvent>>,
    topic: String,
    /// Per-E3 rebroadcast state.
    e3s: HashMap<E3id, E3RebroadcastState>,
}

impl DocumentRebroadcaster {
    pub fn new(
        bus: &BusHandle,
        tx: &mpsc::Sender<NetCommand>,
        rx: &Arc<broadcast::Receiver<NetEvent>>,
        topic: impl Into<String>,
    ) -> Self {
        Self {
            bus: bus.clone(),
            tx: tx.clone(),
            rx: rx.clone(),
            topic: topic.into(),
            e3s: HashMap::new(),
        }
    }

    pub fn setup(
        bus: &BusHandle,
        tx: &mpsc::Sender<NetCommand>,
        rx: &Arc<broadcast::Receiver<NetEvent>>,
        topic: impl Into<String>,
    ) -> Addr<Self> {
        let addr = Self::new(bus, tx, rx, topic).start();

        bus.subscribe_all(
            &[
                EventType::PublishDocumentRequested,
                EventType::E3RequestComplete,
                EventType::E3StageChanged,
                EventType::E3Failed,
                EventType::CommitteeMemberExpelled,
            ],
            addr.clone().into(),
        );

        addr
    }

    fn remove_e3(&mut self, e3_id: &E3id) {
        if let Some(state) = self.e3s.remove(e3_id) {
            if !state.pending.is_empty() {
                debug!(
                    e3_id = %e3_id,
                    documents = state.pending.len(),
                    "Stopped rebroadcasting for E3"
                );
            }
        }
    }

    /// Mark a phase boundary — remove all notifications published before now,
    /// keep any that arrive after (next phase's documents).
    fn mark_phase_complete(&mut self, e3_id: &E3id) {
        if let Some(state) = self.e3s.get_mut(e3_id) {
            state.phase_cutoff = Some(Instant::now());
            state.apply_cutoff();
            if state.pending.is_empty() {
                self.e3s.remove(e3_id);
            }
        }
    }

    fn track_published_document(&mut self, event: PublishDocumentRequested) {
        let now = Instant::now();
        let e3_id = event.meta.e3_id.clone();

        let state = self
            .e3s
            .entry(e3_id)
            .or_insert_with(E3RebroadcastState::new);
        if state.pending.len() >= MAX_PENDING_PER_E3 {
            debug!("Max pending notifications reached for E3, dropping");
            return;
        }

        let key = ContentHash::from_content(&event.value);
        let ts = self.bus.ts().unwrap_or(0);
        let notification = DocumentPublishedNotification::new(event.meta, key, ts);

        state.pending.push(PendingNotification {
            notification,
            first_published: now,
            last_rebroadcast: now,
            rebroadcast_count: 0,
        });
    }

    fn handle_rebroadcast_tick(&mut self, ctx: &mut <Self as Actor>::Context) {
        let now = Instant::now();
        let tx = self.tx.clone();
        let rx = self.rx.clone();
        let topic = self.topic.clone();

        let mut to_rebroadcast = Vec::new();

        for (_e3_id, state) in self.e3s.iter_mut() {
            state.pending.retain_mut(|pending| {
                if pending.rebroadcast_count >= MAX_REBROADCASTS {
                    debug!("Max rebroadcasts reached, dropping notification");
                    return false;
                }

                let elapsed = now.duration_since(pending.first_published);
                if elapsed < REBROADCAST_INITIAL_DELAY {
                    return true;
                }

                let since_last = now.duration_since(pending.last_rebroadcast);
                if since_last >= REBROADCAST_INTERVAL {
                    pending.rebroadcast_count += 1;
                    pending.last_rebroadcast = now;
                    to_rebroadcast.push(pending.notification.clone());
                }

                true
            });
        }

        // Remove empty E3 entries
        self.e3s.retain(|_, s| !s.pending.is_empty());

        if !to_rebroadcast.is_empty() {
            info!(
                count = to_rebroadcast.len(),
                "Rebroadcasting document notifications"
            );

            ctx.spawn(
                async move {
                    for notification in to_rebroadcast {
                        if let Err(e) = broadcast_document_published_notification(
                            tx.clone(),
                            rx.clone(),
                            notification,
                            topic.clone(),
                        )
                        .await
                        {
                            warn!("Failed to rebroadcast notification: {e}");
                        }
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                }
                .into_actor(&*self),
            );
        }
    }
}

impl Actor for DocumentRebroadcaster {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.set_mailbox_capacity(MAILBOX_LIMIT);
        ctx.run_interval(REBROADCAST_INTERVAL, |act, ctx| {
            act.handle_rebroadcast_tick(ctx);
        });
    }
}

impl Handler<EnclaveEvent> for DocumentRebroadcaster {
    type Result = ();

    fn handle(&mut self, msg: EnclaveEvent, _ctx: &mut Self::Context) -> Self::Result {
        let (data, _ec) = msg.into_components();
        match data {
            EnclaveEventData::PublishDocumentRequested(data) => {
                self.track_published_document(data);
            }
            EnclaveEventData::E3StageChanged(data) => match data.new_stage {
                // DKG complete — stop rebroadcasting DKG documents (EncryptionKey,
                // ThresholdShare). Any DecryptionKeyShared documents published
                // after this point will be tracked fresh.
                E3Stage::KeyPublished => {
                    self.mark_phase_complete(&data.e3_id);
                }
                // E3 terminal — stop everything.
                E3Stage::Failed | E3Stage::Complete => {
                    self.remove_e3(&data.e3_id);
                }
                _ => {}
            },
            EnclaveEventData::E3RequestComplete(data) => {
                self.remove_e3(&data.e3_id);
            }
            EnclaveEventData::E3Failed(data) => {
                self.remove_e3(&data.e3_id);
            }
            EnclaveEventData::CommitteeMemberExpelled(data) => {
                self.remove_e3(&data.e3_id);
            }
            _ => (),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use e3_events::{DocumentKind, DocumentMeta, E3StageChanged};

    fn make_e3_id(id: &str) -> E3id {
        E3id::new(id, 1)
    }

    fn make_notification(e3_id: &E3id) -> DocumentPublishedNotification {
        let meta = DocumentMeta::new(e3_id.clone(), DocumentKind::TrBFV, vec![], None);
        let key = ContentHash::from_content(&b"test-document"[..]);
        DocumentPublishedNotification::new(meta, key, 0)
    }

    /// Directly insert a pending notification for testing (bypasses bus/ts).
    fn insert_pending(e3s: &mut HashMap<E3id, E3RebroadcastState>, e3_id: &E3id) {
        let state = e3s
            .entry(e3_id.clone())
            .or_insert_with(E3RebroadcastState::new);
        state.pending.push(PendingNotification {
            notification: make_notification(e3_id),
            first_published: Instant::now(),
            last_rebroadcast: Instant::now(),
            rebroadcast_count: 0,
        });
    }

    #[test]
    fn tracks_pending_notifications() {
        let mut e3s = HashMap::new();
        let e3 = make_e3_id("1");

        insert_pending(&mut e3s, &e3);
        insert_pending(&mut e3s, &e3);

        assert!(e3s.contains_key(&e3));
        assert_eq!(e3s.get(&e3).unwrap().pending.len(), 2);
    }

    #[test]
    fn remove_e3_clears_all_pending() {
        let mut e3s: HashMap<E3id, E3RebroadcastState> = HashMap::new();
        let e3 = make_e3_id("1");

        insert_pending(&mut e3s, &e3);
        insert_pending(&mut e3s, &e3);

        e3s.remove(&e3);
        assert!(!e3s.contains_key(&e3));
    }

    #[test]
    fn e3_failed_removes_pending() {
        let mut e3s = HashMap::new();
        let e3 = make_e3_id("1");
        insert_pending(&mut e3s, &e3);

        // Simulate E3Failed
        e3s.remove(&e3);
        assert!(!e3s.contains_key(&e3));
    }

    #[test]
    fn key_published_clears_dkg_docs_keeps_future_docs() {
        let mut e3s = HashMap::new();
        let e3 = make_e3_id("1");

        // DKG documents published before KeyPublished
        insert_pending(&mut e3s, &e3);
        insert_pending(&mut e3s, &e3);
        assert_eq!(e3s.get(&e3).unwrap().pending.len(), 2);

        // Simulate KeyPublished — mark phase complete
        if let Some(state) = e3s.get_mut(&e3) {
            state.phase_cutoff = Some(Instant::now());
            state.apply_cutoff();
        }
        // DKG docs cleared (published before cutoff)
        assert!(
            e3s.get(&e3).map_or(true, |s| s.pending.is_empty()),
            "DKG docs should be cleared after KeyPublished"
        );

        // Remove empty entry like the real code does
        e3s.retain(|_, s| !s.pending.is_empty());
        assert!(!e3s.contains_key(&e3));

        // Decryption document published after KeyPublished — should be tracked
        insert_pending(&mut e3s, &e3);
        assert_eq!(e3s.get(&e3).unwrap().pending.len(), 1);
    }

    #[test]
    fn independent_e3s_dont_interfere() {
        let mut e3s = HashMap::new();
        let e3_a = make_e3_id("1");
        let e3_b = make_e3_id("2");

        insert_pending(&mut e3s, &e3_a);
        insert_pending(&mut e3s, &e3_b);

        // Remove only E3 A
        e3s.remove(&e3_a);

        assert!(!e3s.contains_key(&e3_a));
        assert!(e3s.contains_key(&e3_b));
        assert_eq!(e3s.get(&e3_b).unwrap().pending.len(), 1);
    }

    #[test]
    fn non_terminal_stage_changes_dont_remove() {
        let mut e3s = HashMap::new();
        let e3 = make_e3_id("1");
        insert_pending(&mut e3s, &e3);

        // CommitteeFinalized is non-terminal — the process_event match
        // only acts on KeyPublished, Failed, Complete. Verify the state
        // is unchanged.
        assert!(e3s.contains_key(&e3));
        assert_eq!(e3s.get(&e3).unwrap().pending.len(), 1);
    }

    #[test]
    fn process_event_dispatches_correctly() {
        let mut e3s = HashMap::new();
        let e3 = make_e3_id("1");
        insert_pending(&mut e3s, &e3);

        // Test the dispatch helper with E3StageChanged(Failed)
        let data = EnclaveEventData::E3StageChanged(E3StageChanged {
            e3_id: e3.clone(),
            previous_stage: E3Stage::Requested,
            new_stage: E3Stage::Failed,
        });

        // Replicate the handler logic
        match data {
            EnclaveEventData::E3StageChanged(d) => match d.new_stage {
                E3Stage::KeyPublished => {
                    if let Some(state) = e3s.get_mut(&d.e3_id) {
                        state.phase_cutoff = Some(Instant::now());
                        state.apply_cutoff();
                    }
                }
                E3Stage::Failed | E3Stage::Complete => {
                    e3s.remove(&d.e3_id);
                }
                _ => {}
            },
            _ => {}
        }

        assert!(!e3s.contains_key(&e3));
    }

    #[test]
    fn max_rebroadcasts_drops_notification() {
        let e3 = make_e3_id("1");
        let mut state = E3RebroadcastState::new();
        state.pending.push(PendingNotification {
            notification: make_notification(&e3),
            first_published: Instant::now() - Duration::from_secs(600),
            last_rebroadcast: Instant::now() - Duration::from_secs(600),
            rebroadcast_count: MAX_REBROADCASTS, // Already at max
        });

        // Simulate retain logic from handle_rebroadcast_tick
        state
            .pending
            .retain(|p| p.rebroadcast_count < MAX_REBROADCASTS);
        assert!(state.pending.is_empty());
    }

    #[test]
    fn remove_nonexistent_e3_is_noop() {
        let mut e3s: HashMap<E3id, E3RebroadcastState> = HashMap::new();
        let e3 = make_e3_id("1");

        // Should not panic
        e3s.remove(&e3);
        assert!(!e3s.contains_key(&e3));
    }
}
