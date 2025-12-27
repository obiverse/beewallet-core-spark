//! Spark SDK Integration Layer
//!
//! Clean wrapper around `breez-sdk-spark` that bridges to our 9S reactive system.
//!
//! ## Design Principles
//!
//! 1. **Thin wrapper** - Minimal abstraction over SDK, just type mapping
//! 2. **Async all the way** - SDK is async, we expose async
//! 3. **Event bridge** - SDK SdkEvent → our SdkEvent → Reactor → Scrolls
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    SparkSdkWrapper                           │
//! │  ┌────────────────┐  ┌─────────────────────────────────────┐│
//! │  │  BreezSdk      │  │  Event Subscription                  ││
//! │  │  (SDK handle)  │  │  add_event_listener() → bridge       ││
//! │  └───────┬────────┘  └──────────────┬──────────────────────┘│
//! │          │                          │                        │
//! │          ▼                          ▼                        │
//! │  ┌────────────────────────────────────────────────────────┐ │
//! │  │              WalletReactor                              │ │
//! │  │  SDK SdkEvent → our SdkEvent → ingest() → Scroll       │ │
//! │  └────────────────────────────────────────────────────────┘ │
//! └─────────────────────────────────────────────────────────────┘
//! ```

use std::sync::Arc;
use tokio::runtime::Handle;
use tokio::sync::RwLock;

use breez_sdk_spark::{
    BreezSdk,
    ConnectRequest,
    EventListener as SdkEventListener,
    GetInfoRequest,
    ListPaymentsRequest,
    Network as SdkNetwork,
    Payment as SdkPayment,
    PaymentMethod as SdkPaymentMethod,
    PaymentStatus as SdkPaymentStatus,
    PaymentType as SdkPaymentType,
    ReceivePaymentMethod,
    ReceivePaymentRequest,
    PrepareSendPaymentRequest,
    SendPaymentRequest,
    SdkError,
    SdkEvent as SdkSdkEvent,
    Seed,
    SyncWalletRequest,
    connect,
    default_config,
};

use super::{
    SparkNetwork,
    events::{SdkEvent, Payment, PaymentType, PaymentState, PaymentDetails},
    reactor::WalletReactor,
};

/// Spark SDK wrapper that bridges to our reactive system
pub struct SparkSdkWrapper {
    /// The underlying Breez SDK
    sdk: Arc<RwLock<Option<Arc<BreezSdk>>>>,
    /// Reactor to push events to
    reactor: Arc<WalletReactor>,
    /// Tokio runtime handle
    #[allow(dead_code)]
    runtime: Handle,
}

impl SparkSdkWrapper {
    /// Create a new SDK wrapper
    pub fn new(reactor: Arc<WalletReactor>, runtime: Handle) -> Self {
        Self {
            sdk: Arc::new(RwLock::new(None)),
            reactor,
            runtime,
        }
    }

    /// Connect to Spark network
    ///
    /// Creates a BreezSdk from mnemonic and starts event subscription.
    pub async fn connect(
        &self,
        mnemonic: &str,
        passphrase: Option<&str>,
        network: SparkNetwork,
        working_dir: &str,
    ) -> Result<(), SdkError> {
        // Get default config for network
        let config = default_config(network.into());

        // Create connect request
        let request = ConnectRequest {
            config,
            seed: Seed::Mnemonic {
                mnemonic: mnemonic.to_string(),
                passphrase: passphrase.map(|s| s.to_string()),
            },
            storage_dir: working_dir.to_string(),
        };

        // Connect
        let sdk = connect(request).await?;
        let sdk = Arc::new(sdk);

        // Register event listener
        let listener = ReactorBridge::new(self.reactor.clone());
        let _listener_id = sdk.add_event_listener(Box::new(listener)).await;

        // Store SDK
        let mut guard = self.sdk.write().await;
        *guard = Some(sdk);

        // Emit synced event
        self.reactor.ingest(SdkEvent::Synced);

        Ok(())
    }

    /// Disconnect from Spark network
    pub async fn disconnect(&self) -> Result<(), SdkError> {
        let mut guard = self.sdk.write().await;
        if let Some(sdk) = guard.take() {
            sdk.disconnect().await?;
        }
        Ok(())
    }

    /// Check if connected
    pub async fn is_connected(&self) -> bool {
        self.sdk.read().await.is_some()
    }

    // =========================================================================
    // Read Operations
    // =========================================================================

    /// Get wallet balance in satoshis
    pub async fn get_balance(&self) -> Result<u64, SdkError> {
        let guard = self.sdk.read().await;
        let sdk = guard.as_ref().ok_or_else(|| SdkError::Generic("Not connected".into()))?;

        let info = sdk.get_info(GetInfoRequest { ensure_synced: Some(false) }).await?;
        Ok(info.balance_sats)
    }

    /// Get Spark address for receiving
    ///
    /// This is the wallet's identity address - payments can be received here.
    pub async fn get_spark_address(&self) -> Result<String, SdkError> {
        let guard = self.sdk.read().await;
        let sdk = guard.as_ref().ok_or_else(|| SdkError::Generic("Not connected".into()))?;

        // Use receive_payment with SparkAddress to get the address
        let response = sdk.receive_payment(ReceivePaymentRequest {
            payment_method: ReceivePaymentMethod::SparkAddress,
        }).await?;

        Ok(response.payment_request)
    }

    /// List recent payments
    pub async fn list_payments(&self, limit: Option<u32>) -> Result<Vec<PaymentInfo>, SdkError> {
        let guard = self.sdk.read().await;
        let sdk = guard.as_ref().ok_or_else(|| SdkError::Generic("Not connected".into()))?;

        let response = sdk.list_payments(ListPaymentsRequest {
            limit,
            ..Default::default()
        }).await?;

        Ok(response.payments.into_iter().map(PaymentInfo::from).collect())
    }

    // =========================================================================
    // Write Operations
    // =========================================================================

    /// Create a Spark invoice for receiving payment
    pub async fn create_invoice(
        &self,
        amount_sat: u64,
        description: Option<String>,
    ) -> Result<ReceiveInfo, SdkError> {
        let guard = self.sdk.read().await;
        let sdk = guard.as_ref().ok_or_else(|| SdkError::Generic("Not connected".into()))?;

        let response = sdk.receive_payment(ReceivePaymentRequest {
            payment_method: ReceivePaymentMethod::SparkInvoice {
                amount: Some(amount_sat as u128),
                token_identifier: None,
                expiry_time: None,
                description,
                sender_public_key: None,
            },
        }).await?;

        Ok(ReceiveInfo {
            destination: response.payment_request,
            fee_sat: response.fee as u64,
        })
    }

    /// Send payment (two-step: prepare then send)
    pub async fn send(
        &self,
        destination: &str,
        amount_sat: Option<u64>,
    ) -> Result<PaymentInfo, SdkError> {
        let guard = self.sdk.read().await;
        let sdk = guard.as_ref().ok_or_else(|| SdkError::Generic("Not connected".into()))?;

        // Step 1: Prepare the payment
        let prepare_response = sdk.prepare_send_payment(PrepareSendPaymentRequest {
            payment_request: destination.to_string(),
            amount: amount_sat.map(|a| a as u128),
            token_identifier: None,
        }).await?;

        // Step 2: Send the payment
        let response = sdk.send_payment(SendPaymentRequest {
            prepare_response,
            options: None,
            idempotency_key: None,
        }).await?;

        Ok(PaymentInfo::from(response.payment))
    }

    /// Sync wallet state with the network
    pub async fn sync(&self) -> Result<(), SdkError> {
        let guard = self.sdk.read().await;
        let sdk = guard.as_ref().ok_or_else(|| SdkError::Generic("Not connected".into()))?;
        sdk.sync_wallet(SyncWalletRequest {}).await?;
        Ok(())
    }
}

// ============================================================================
// Event Bridge - Connects SDK events to our Reactor
// ============================================================================

/// Bridge that forwards SDK events to our WalletReactor
struct ReactorBridge {
    reactor: Arc<WalletReactor>,
}

impl ReactorBridge {
    fn new(reactor: Arc<WalletReactor>) -> Self {
        Self { reactor }
    }
}

#[async_trait::async_trait]
impl SdkEventListener for ReactorBridge {
    async fn on_event(&self, event: SdkSdkEvent) {
        // Convert SDK event to our SdkEvent and ingest into reactor
        if let Some(our_event) = convert_sdk_event(event) {
            self.reactor.ingest(our_event);
        }
    }
}

/// Convert SDK SdkEvent to our SdkEvent
fn convert_sdk_event(event: SdkSdkEvent) -> Option<SdkEvent> {
    match event {
        SdkSdkEvent::Synced => Some(SdkEvent::Synced),

        SdkSdkEvent::PaymentSucceeded { payment } => {
            Some(SdkEvent::PaymentSucceeded {
                payment: convert_payment(payment),
            })
        }

        SdkSdkEvent::PaymentPending { payment } => {
            Some(SdkEvent::PaymentPending {
                payment: convert_payment(payment),
            })
        }

        SdkSdkEvent::PaymentFailed { payment } => {
            Some(SdkEvent::PaymentFailed {
                payment: convert_payment(payment),
            })
        }

        // Other events we don't handle yet
        _ => None,
    }
}

/// Convert SDK Payment to our Payment
fn convert_payment(p: SdkPayment) -> Payment {
    Payment {
        id: p.id,
        payment_type: match p.payment_type {
            SdkPaymentType::Receive => PaymentType::Receive,
            SdkPaymentType::Send => PaymentType::Send,
        },
        state: match p.status {
            SdkPaymentStatus::Pending => PaymentState::Pending,
            SdkPaymentStatus::Completed => PaymentState::Complete,
            SdkPaymentStatus::Failed => PaymentState::Failed,
        },
        amount_sat: p.amount as u64,
        fee_sat: Some(p.fees as u64),
        timestamp: Some(p.timestamp),
        description: None, // SDK Payment doesn't have description at top level
        details: convert_payment_details(p.method, p.details),
    }
}

/// Convert SDK payment method/details to our PaymentDetails
fn convert_payment_details(
    method: SdkPaymentMethod,
    details: Option<breez_sdk_spark::PaymentDetails>,
) -> PaymentDetails {
    // Use method to determine the type if details are missing
    match details {
        Some(breez_sdk_spark::PaymentDetails::Lightning { invoice, preimage, .. }) => {
            PaymentDetails::Lightning {
                swap_id: None,
                bolt11: Some(invoice),
                preimage,
            }
        }
        Some(breez_sdk_spark::PaymentDetails::Spark { .. }) => {
            PaymentDetails::Spark {
                transfer_id: None,
                spark_address: None,
            }
        }
        Some(breez_sdk_spark::PaymentDetails::Token { .. }) => {
            // Token payments - treat as Spark for now
            PaymentDetails::Spark {
                transfer_id: None,
                spark_address: None,
            }
        }
        Some(breez_sdk_spark::PaymentDetails::Withdraw { .. }) |
        Some(breez_sdk_spark::PaymentDetails::Deposit { .. }) => {
            // On-chain deposit/withdraw - treat as Bitcoin
            PaymentDetails::Bitcoin {
                txid: None,
                confirmations: None,
            }
        }
        None => {
            // Fallback based on method
            match method {
                SdkPaymentMethod::Lightning => PaymentDetails::Lightning {
                    swap_id: None,
                    bolt11: None,
                    preimage: None,
                },
                SdkPaymentMethod::Spark | SdkPaymentMethod::Token => PaymentDetails::Spark {
                    transfer_id: None,
                    spark_address: None,
                },
                SdkPaymentMethod::Deposit | SdkPaymentMethod::Withdraw => PaymentDetails::Bitcoin {
                    txid: None,
                    confirmations: None,
                },
                _ => PaymentDetails::default(),
            }
        }
    }
}

// ============================================================================
// Response Types
// ============================================================================

/// Payment info from SDK
#[derive(Debug, Clone)]
pub struct PaymentInfo {
    pub id: String,
    pub payment_type: PaymentType,
    pub status: PaymentState,
    pub amount_sat: u64,
    pub fee_sat: Option<u64>,
    pub timestamp: Option<u64>,
    pub description: Option<String>,
}

impl From<SdkPayment> for PaymentInfo {
    fn from(p: SdkPayment) -> Self {
        PaymentInfo {
            id: p.id,
            payment_type: match p.payment_type {
                SdkPaymentType::Receive => PaymentType::Receive,
                SdkPaymentType::Send => PaymentType::Send,
            },
            status: match p.status {
                SdkPaymentStatus::Pending => PaymentState::Pending,
                SdkPaymentStatus::Completed => PaymentState::Complete,
                SdkPaymentStatus::Failed => PaymentState::Failed,
            },
            amount_sat: p.amount as u64,
            fee_sat: Some(p.fees as u64),
            timestamp: Some(p.timestamp),
            description: None,
        }
    }
}

/// Receive info from SDK
#[derive(Debug, Clone)]
pub struct ReceiveInfo {
    pub destination: String,
    pub fee_sat: u64,
}

// ============================================================================
// Network Conversion
// ============================================================================

impl From<SparkNetwork> for SdkNetwork {
    fn from(n: SparkNetwork) -> Self {
        match n {
            SparkNetwork::Mainnet => SdkNetwork::Mainnet,
            // Spark SDK only has Mainnet and Regtest
            SparkNetwork::Testnet => SdkNetwork::Regtest,
            SparkNetwork::Regtest => SdkNetwork::Regtest,
        }
    }
}

impl From<SdkNetwork> for SparkNetwork {
    fn from(n: SdkNetwork) -> Self {
        match n {
            SdkNetwork::Mainnet => SparkNetwork::Mainnet,
            SdkNetwork::Regtest => SparkNetwork::Regtest,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_conversion() {
        // Test our SparkNetwork -> SdkNetwork conversion
        // (Can't use assert_eq! because SdkNetwork doesn't impl PartialEq)
        let regtest: SdkNetwork = SparkNetwork::Regtest.into();
        match regtest {
            SdkNetwork::Regtest => (),
            _ => panic!("Expected Regtest"),
        }

        // Test SdkNetwork -> SparkNetwork conversion
        let mainnet: SparkNetwork = SdkNetwork::Mainnet.into();
        assert_eq!(mainnet, SparkNetwork::Mainnet);

        // Test Testnet maps to Regtest (Spark SDK only has Mainnet/Regtest)
        let testnet: SdkNetwork = SparkNetwork::Testnet.into();
        match testnet {
            SdkNetwork::Regtest => (),
            _ => panic!("Expected Testnet to map to Regtest"),
        }
    }
}
