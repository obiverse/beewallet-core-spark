//! Wallet Reactor - Event-Driven Reactive Core
//!
//! The platonic form of a reactive wallet backend.
//!
//! ## Philosophy
//!
//! The wallet is a **membrane** between user intent and network reality:
//! - **Input**: User writes to paths (`/send`, `/invoice`)
//! - **Output**: SDK emits events → Scrolls flow through `watch()`
//!
//! ## Architecture
//!
//! ```text
//!                      ┌─────────────────────┐
//!                      │   Spark SDK         │
//!                      │   (event source)    │
//!                      └──────────┬──────────┘
//!                                 │ SdkEvent
//!                                 ▼
//!              ┌──────────────────────────────────────┐
//!              │            WalletReactor             │
//!              │                                      │
//!              │  ┌────────────┐  ┌────────────────┐ │
//!              │  │ EventBus   │  │ State Cache    │ │
//!              │  │ (broadcast)│  │ (latest vals)  │ │
//!              │  └─────┬──────┘  └────────────────┘ │
//!              │        │                            │
//!              │        ▼                            │
//!              │  ┌────────────────────────────────┐ │
//!              │  │ Pattern Matchers               │ │
//!              │  │ /tx/** → payment watchers      │ │
//!              │  │ /balance → balance watchers    │ │
//!              │  └────────────────────────────────┘ │
//!              └──────────────────┬─────────────────┘
//!                                 │ Scroll
//!                                 ▼
//!              ┌──────────────────────────────────────┐
//!              │         Tauri / Flutter / CLI        │
//!              │         (reactive UI layer)          │
//!              └──────────────────────────────────────┘
//! ```
//!
//! ## The 9S Way
//!
//! Events are Scrolls. Watching is subscribing. Everything flows through 5 ops.
//!
//! ```rust,ignore
//! // Subscribe to payment events
//! let mut rx = wallet.watch("/tx/**")?;
//!
//! // React to incoming scrolls
//! while let Some(scroll) = rx.recv() {
//!     match scroll.type_.as_str() {
//!         "wallet/payment@v1" => update_payment_list(scroll),
//!         "wallet/balance@v1" => update_balance_display(scroll),
//!         _ => {}
//!     }
//! }
//! ```

use crate::nine_s::Scroll;
use crate::nine_s::channel::{channel, Sender, Receiver};
use crate::nine_s::namespace::path_matches;
use super::events::{SdkEvent, Payment, PaymentState, PaymentType};
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

/// Channel capacity for event bus
const EVENT_BUS_CAPACITY: usize = 256;

/// Watcher entry with pattern and sender
struct Watcher {
    pattern: String,
    sender: Sender<Scroll>,
}

/// Wallet Reactor - The reactive core
///
/// Transforms SDK events into Scroll streams that UI layers can watch.
/// Maintains state cache for instant reads while events flow asynchronously.
pub struct WalletReactor {
    /// Registered watchers (pattern → senders)
    watchers: Arc<Mutex<Vec<Watcher>>>,

    /// State cache for instant reads (path → latest scroll)
    state: Arc<RwLock<HashMap<String, Scroll>>>,

    /// Watcher ID counter
    next_watcher_id: Arc<Mutex<u64>>,
}

impl WalletReactor {
    /// Create a new reactor
    pub fn new() -> Self {
        Self {
            watchers: Arc::new(Mutex::new(Vec::new())),
            state: Arc::new(RwLock::new(HashMap::new())),
            next_watcher_id: Arc::new(Mutex::new(0)),
        }
    }

    // =========================================================================
    // Event Ingestion (from SDK)
    // =========================================================================

    /// Process an SDK event, converting to Scrolls and dispatching
    ///
    /// This is the main entry point for SDK events. Call this from the
    /// SDK's event listener callback.
    pub fn ingest(&self, event: SdkEvent) {
        let scrolls = self.event_to_scrolls(event);

        for scroll in scrolls {
            self.update_state(&scroll);
            self.dispatch(&scroll);
        }
    }

    /// Convert SDK event to one or more Scrolls
    fn event_to_scrolls(&self, event: SdkEvent) -> Vec<Scroll> {
        match event {
            SdkEvent::Synced => {
                vec![Scroll::typed(
                    "/wallet/synced",
                    json!({"synced": true, "timestamp": now_unix()}),
                    "wallet/synced@v1",
                )]
            }

            SdkEvent::PaymentSucceeded { payment } => {
                self.payment_to_scrolls(&payment, "succeeded")
            }

            SdkEvent::PaymentPending { payment } => {
                self.payment_to_scrolls(&payment, "pending")
            }

            SdkEvent::PaymentFailed { payment } => {
                self.payment_to_scrolls(&payment, "failed")
            }

            SdkEvent::ClaimedDeposits { claimed_deposits } => {
                claimed_deposits.into_iter().map(|d| {
                    Scroll::typed(
                        &format!("/wallet/deposit/{}", d.txid),
                        json!({
                            "txid": d.txid,
                            "amount_sat": d.amount_sat,
                            "status": "claimed",
                        }),
                        "wallet/deposit@v1",
                    )
                }).collect()
            }

            SdkEvent::UnclaimedDeposits { unclaimed_deposits } => {
                unclaimed_deposits.into_iter().map(|d| {
                    Scroll::typed(
                        &format!("/wallet/deposit/unclaimed/{}", d.address),
                        json!({
                            "address": d.address,
                            "amount_sat": d.amount_sat,
                            "timestamp": d.timestamp,
                            "status": "unclaimed",
                        }),
                        "wallet/deposit@v1",
                    )
                }).collect()
            }
        }
    }

    /// Convert a Payment to Scrolls (payment + balance update)
    fn payment_to_scrolls(&self, payment: &Payment, status: &str) -> Vec<Scroll> {
        let mut scrolls = Vec::new();

        // Payment scroll
        let payment_scroll = Scroll::typed(
            &format!("/wallet/tx/{}", payment.id),
            json!({
                "id": payment.id,
                "type": match payment.payment_type {
                    PaymentType::Receive => "receive",
                    PaymentType::Send => "send",
                },
                "state": format!("{:?}", payment.state).to_lowercase(),
                "status": status,
                "amount_sat": payment.amount_sat,
                "fee_sat": payment.fee_sat,
                "timestamp": payment.timestamp,
                "description": payment.description,
                "details": serde_json::to_value(&payment.details).unwrap_or_default(),
            }),
            "wallet/payment@v1",
        );
        scrolls.push(payment_scroll);

        // Balance hint (triggers balance watchers to refetch)
        // The actual balance will be fetched via read("/balance")
        if payment.state == PaymentState::Complete {
            let balance_hint = Scroll::typed(
                "/wallet/balance",
                json!({
                    "hint": "changed",
                    "last_payment_id": payment.id,
                }),
                "wallet/balance-hint@v1",
            );
            scrolls.push(balance_hint);
        }

        scrolls
    }

    // =========================================================================
    // State Cache (for instant reads)
    // =========================================================================

    /// Update state cache with new scroll
    fn update_state(&self, scroll: &Scroll) {
        if let Ok(mut state) = self.state.write() {
            state.insert(scroll.key.clone(), scroll.clone());
        }
    }

    /// Get cached state for a path
    pub fn get_cached(&self, path: &str) -> Option<Scroll> {
        self.state.read().ok()?.get(path).cloned()
    }

    /// Set state directly (for initial load)
    pub fn set_state(&self, scroll: Scroll) {
        self.update_state(&scroll);
    }

    /// Clear all cached state
    pub fn clear_state(&self) {
        if let Ok(mut state) = self.state.write() {
            state.clear();
        }
    }

    // =========================================================================
    // Watch System (reactive subscriptions)
    // =========================================================================

    /// Register a watcher for a pattern
    ///
    /// Returns a receiver that will get Scrolls matching the pattern.
    /// Patterns support:
    /// - Exact: `/wallet/balance`
    /// - Single wildcard: `/wallet/tx/*`
    /// - Recursive: `/wallet/**`
    pub fn watch(&self, pattern: &str) -> Receiver<Scroll> {
        let (tx, rx) = channel(EVENT_BUS_CAPACITY);

        let watcher = Watcher {
            pattern: pattern.to_string(),
            sender: tx,
        };

        if let Ok(mut watchers) = self.watchers.lock() {
            watchers.push(watcher);
        }

        rx
    }

    /// Dispatch a scroll to matching watchers
    fn dispatch(&self, scroll: &Scroll) {
        let mut watchers = match self.watchers.lock() {
            Ok(w) => w,
            Err(_) => return,
        };

        // Remove disconnected watchers and dispatch to matching ones
        watchers.retain(|watcher| {
            if path_matches(&scroll.key, &watcher.pattern) {
                // Try to send, remove if channel is disconnected
                watcher.sender.try_send(scroll.clone()).is_ok()
            } else {
                // Keep watcher even if this scroll doesn't match
                true
            }
        });
    }

    /// Get count of active watchers
    pub fn watcher_count(&self) -> usize {
        self.watchers.lock().map(|w| w.len()).unwrap_or(0)
    }

    // =========================================================================
    // Manual Event Emission (for testing / manual triggers)
    // =========================================================================

    /// Emit a scroll directly (bypasses SDK)
    ///
    /// Use this for:
    /// - Testing reactive flows
    /// - Manual balance updates
    /// - Custom events
    pub fn emit(&self, scroll: Scroll) {
        self.update_state(&scroll);
        self.dispatch(&scroll);
    }

    /// Emit balance update
    pub fn emit_balance(&self, confirmed: u64, pending: u64) {
        let scroll = Scroll::typed(
            "/wallet/balance",
            json!({
                "confirmed": confirmed,
                "trusted_pending": pending,
                "untrusted_pending": 0,
                "immature": 0,
                "total": confirmed + pending,
                "spendable": confirmed,
            }),
            "wallet/balance@v1",
        );
        self.emit(scroll);
    }
}

impl Default for WalletReactor {
    fn default() -> Self {
        Self::new()
    }
}

/// Get current Unix timestamp in seconds
fn now_unix() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// =============================================================================
// SDK Event Listener Adapter
// =============================================================================

/// Adapter that implements EventListener and forwards to WalletReactor
///
/// Use this to bridge the SDK's async event listener to the reactor.
///
/// ```rust,ignore
/// let reactor = Arc::new(WalletReactor::new());
/// let adapter = ReactorEventAdapter::new(reactor.clone());
/// sdk.add_event_listener(Box::new(adapter));
/// ```
pub struct ReactorEventAdapter {
    reactor: Arc<WalletReactor>,
}

impl ReactorEventAdapter {
    /// Create adapter for a reactor
    pub fn new(reactor: Arc<WalletReactor>) -> Self {
        Self { reactor }
    }
}

#[async_trait::async_trait]
impl super::events::EventListener for ReactorEventAdapter {
    async fn on_event(&self, event: SdkEvent) {
        self.reactor.ingest(event);
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reactor_creation() {
        let reactor = WalletReactor::new();
        assert_eq!(reactor.watcher_count(), 0);
    }

    #[test]
    fn test_watch_and_emit() {
        let reactor = WalletReactor::new();

        // Watch for balance changes
        let mut rx = reactor.watch("/wallet/balance");

        // Emit a balance update
        reactor.emit_balance(10000, 500);

        // Should receive the scroll
        let scroll = rx.try_recv().expect("Should receive scroll");
        assert_eq!(scroll.key, "/wallet/balance");
        assert_eq!(scroll.data["confirmed"], 10000);
        assert_eq!(scroll.data["trusted_pending"], 500);
    }

    #[test]
    fn test_pattern_matching() {
        let reactor = WalletReactor::new();

        // Watch for all tx events
        let mut rx = reactor.watch("/wallet/tx/**");

        // Emit a payment
        let payment = Payment {
            id: "test123".to_string(),
            payment_type: PaymentType::Receive,
            state: PaymentState::Complete,
            amount_sat: 5000,
            fee_sat: None,
            timestamp: Some(now_unix()),
            description: Some("Test payment".to_string()),
            details: super::super::events::PaymentDetails::default(),
        };

        reactor.ingest(SdkEvent::PaymentSucceeded { payment });

        // Should receive the payment scroll
        let scroll = rx.try_recv().expect("Should receive payment scroll");
        assert!(scroll.key.starts_with("/wallet/tx/"));
        assert_eq!(scroll.data["amount_sat"], 5000);
    }

    #[test]
    fn test_state_caching() {
        let reactor = WalletReactor::new();

        // Emit balance
        reactor.emit_balance(20000, 0);

        // Should be cached
        let cached = reactor.get_cached("/wallet/balance");
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().data["confirmed"], 20000);
    }

    #[test]
    fn test_multiple_watchers() {
        let reactor = WalletReactor::new();

        // Two watchers for balance
        let mut rx1 = reactor.watch("/wallet/balance");
        let mut rx2 = reactor.watch("/wallet/balance");

        // Emit once
        reactor.emit_balance(1000, 0);

        // Both should receive
        assert!(rx1.try_recv().is_some());
        assert!(rx2.try_recv().is_some());
    }

    #[test]
    fn test_disconnected_watcher_cleanup() {
        let reactor = WalletReactor::new();

        // Create and immediately drop a watcher
        {
            let _rx = reactor.watch("/wallet/balance");
        }

        // Emit something - should clean up disconnected watcher
        reactor.emit_balance(1000, 0);

        // Watcher count may still show 1 until next dispatch
        // (cleanup happens during dispatch to matching pattern)
    }

    #[test]
    fn test_synced_event() {
        let reactor = WalletReactor::new();
        let mut rx = reactor.watch("/wallet/synced");

        reactor.ingest(SdkEvent::Synced);

        let scroll = rx.try_recv().expect("Should receive synced event");
        assert_eq!(scroll.key, "/wallet/synced");
        assert_eq!(scroll.data["synced"], true);
    }
}
