//! Event types for payment streaming
//!
//! Provides EventListener trait and SdkEvent enum for real-time
//! payment notifications. Mirrors Breez SDK Liquid API for compatibility.

use serde::{Deserialize, Serialize};

/// Payment event listener trait
///
/// Implement this to receive real-time payment notifications.
/// Register via `WalletManager::add_event_listener()`.
#[async_trait::async_trait]
pub trait EventListener: Send + Sync {
    /// Called when an SDK event occurs
    async fn on_event(&self, event: SdkEvent);
}

/// SDK events for payment state changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SdkEvent {
    /// Wallet has synced with the network
    Synced,
    /// Payment completed successfully
    PaymentSucceeded { payment: Payment },
    /// Payment is pending confirmation
    PaymentPending { payment: Payment },
    /// Payment failed
    PaymentFailed { payment: Payment },
    /// Deposits have been claimed
    ClaimedDeposits { claimed_deposits: Vec<ClaimedDeposit> },
    /// Deposits are awaiting claim (manual action may be needed)
    UnclaimedDeposits { unclaimed_deposits: Vec<UnclaimedDeposit> },
}

/// Payment details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    /// Payment identifier (tx_id or swap_id)
    pub id: String,
    /// Payment type
    pub payment_type: PaymentType,
    /// Payment state
    pub state: PaymentState,
    /// Amount in satoshis
    pub amount_sat: u64,
    /// Fee in satoshis (if known)
    pub fee_sat: Option<u64>,
    /// Timestamp (unix seconds)
    pub timestamp: Option<u64>,
    /// Description or memo
    pub description: Option<String>,
    /// Additional payment details
    pub details: PaymentDetails,
}

/// Payment type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentType {
    /// Incoming payment
    Receive,
    /// Outgoing payment
    Send,
}

/// Payment state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentState {
    /// Payment created, not yet broadcast
    Created,
    /// Payment pending confirmation
    Pending,
    /// Payment completed successfully
    Complete,
    /// Payment failed
    Failed,
    /// Payment timed out
    TimedOut,
    /// Payment is refundable
    Refundable,
    /// Refund is pending
    RefundPending,
}

/// Detailed payment information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaymentDetails {
    /// Lightning payment (via Spark or submarine swap)
    Lightning {
        /// Swap ID for tracking
        swap_id: Option<String>,
        /// BOLT11 invoice
        bolt11: Option<String>,
        /// Preimage (proof of payment)
        preimage: Option<String>,
    },
    /// Bitcoin on-chain payment
    Bitcoin {
        /// Transaction ID
        txid: Option<String>,
        /// Confirmations
        confirmations: Option<u32>,
    },
    /// Spark-to-Spark transfer
    Spark {
        /// Transfer ID
        transfer_id: Option<String>,
        /// Spark address
        spark_address: Option<String>,
    },
}

impl Default for PaymentDetails {
    fn default() -> Self {
        PaymentDetails::Lightning {
            swap_id: None,
            bolt11: None,
            preimage: None,
        }
    }
}

/// A deposit that has been claimed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimedDeposit {
    /// Transaction ID of the deposit
    pub txid: String,
    /// Amount claimed in satoshis
    pub amount_sat: u64,
}

/// A deposit awaiting claim
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnclaimedDeposit {
    /// Bitcoin address where funds were sent
    pub address: String,
    /// Amount in satoshis
    pub amount_sat: u64,
    /// Timestamp when deposit was detected
    pub timestamp: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payment_type_serialization() {
        let pt = PaymentType::Receive;
        let json = serde_json::to_string(&pt).unwrap();
        assert!(json.contains("Receive"));
    }

    #[test]
    fn test_sdk_event_serialization() {
        let event = SdkEvent::Synced;
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("Synced"));
    }

    #[test]
    fn test_payment_details_default() {
        let details = PaymentDetails::default();
        match details {
            PaymentDetails::Lightning { .. } => (),
            _ => panic!("Expected Lightning variant"),
        }
    }
}
