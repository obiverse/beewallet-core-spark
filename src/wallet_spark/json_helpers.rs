//! JSON conversion helpers for WalletNamespace
//!
//! Pure functions that transform wallet types to JSON.
//! No business logic, no side effects.

use super::{TransactionDetails, WalletBalance};
use serde_json::{json, Value};

/// Convert WalletBalance to JSON
pub fn balance_to_json(balance: &WalletBalance) -> Value {
    json!({
        "confirmed": balance.confirmed,
        "immature": balance.immature,
        "trusted_pending": balance.trusted_pending,
        "untrusted_pending": balance.untrusted_pending,
        "pending": balance.trusted_pending + balance.untrusted_pending,
        "spendable": balance.spendable(),
        "total": balance.total(),
    })
}

/// Convert TransactionDetails to JSON
pub fn tx_to_json(tx: &TransactionDetails) -> Value {
    json!({
        "txid": tx.txid,
        "received": tx.received,
        "sent": tx.sent,
        "fee": tx.fee,
        "is_confirmed": tx.is_confirmed,
        "timestamp": tx.timestamp,
        "confirmation_time": tx.confirmation_time,
        "vsize": tx.vsize,
    })
}

/// Convert a list of transactions to JSON array
pub fn txs_to_json(txs: &[TransactionDetails]) -> Value {
    let tx_array: Vec<Value> = txs.iter().map(tx_to_json).collect();
    Value::Array(tx_array)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_balance_to_json() {
        let balance = WalletBalance {
            confirmed: 10000,
            immature: 0,
            trusted_pending: 5000,
            untrusted_pending: 0,
        };

        let json = balance_to_json(&balance);

        assert_eq!(json["confirmed"], 10000);
        assert_eq!(json["trusted_pending"], 5000);
        assert_eq!(json["pending"], 5000);
        assert_eq!(json["spendable"], 10000);
        assert_eq!(json["total"], 15000);
    }

    #[test]
    fn test_tx_to_json() {
        let tx = TransactionDetails {
            txid: "abc123".to_string(),
            received: 5000,
            sent: 0,
            fee: Some(200),
            confirmation_time: Some(1700000000),
            is_confirmed: true,
            vsize: Some(140),
            timestamp: Some(1700000000),
        };

        let json = tx_to_json(&tx);

        assert_eq!(json["txid"], "abc123");
        assert_eq!(json["received"], 5000);
        assert_eq!(json["is_confirmed"], true);
    }

    #[test]
    fn test_txs_to_json() {
        let txs = vec![
            TransactionDetails {
                txid: "tx1".to_string(),
                received: 1000,
                sent: 0,
                fee: None,
                confirmation_time: None,
                is_confirmed: false,
                vsize: None,
                timestamp: None,
            },
            TransactionDetails {
                txid: "tx2".to_string(),
                received: 0,
                sent: 500,
                fee: Some(100),
                confirmation_time: Some(1700000000),
                is_confirmed: true,
                vsize: Some(100),
                timestamp: Some(1700000000),
            },
        ];

        let json = txs_to_json(&txs);

        assert!(json.is_array());
        assert_eq!(json.as_array().unwrap().len(), 2);
    }
}
