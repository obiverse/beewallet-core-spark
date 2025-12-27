//! Platform-agnostic channel abstraction for Watch
//!
//! Uses std::sync::mpsc - no async runtime dependency.
//! This allows beewallet-core to work in any environment.

use std::sync::mpsc as std_mpsc;

/// Bounded channel sender
pub struct Sender<T> {
    inner: std_mpsc::SyncSender<T>,
}

/// Bounded channel receiver
pub struct Receiver<T> {
    inner: std_mpsc::Receiver<T>,
}

/// Create a bounded channel with the given capacity
pub fn channel<T>(capacity: usize) -> (Sender<T>, Receiver<T>) {
    let (tx, rx) = std_mpsc::sync_channel(capacity);
    (Sender { inner: tx }, Receiver { inner: rx })
}

impl<T> Sender<T> {
    /// Try to send a value without blocking
    /// Returns Err with the value if the channel is full or disconnected
    pub fn try_send(&self, value: T) -> Result<(), T> {
        self.inner.try_send(value).map_err(|e| match e {
            std_mpsc::TrySendError::Full(v) => v,
            std_mpsc::TrySendError::Disconnected(v) => v,
        })
    }

    /// Send a value, blocking if the channel is full
    pub fn send(&self, value: T) -> Result<(), T> {
        self.inner.send(value).map_err(|e| e.0)
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> Receiver<T> {
    /// Receive a value, blocking until one is available
    pub fn recv(&mut self) -> Option<T> {
        self.inner.recv().ok()
    }

    /// Try to receive without blocking
    pub fn try_recv(&mut self) -> Option<T> {
        self.inner.try_recv().ok()
    }

    /// Iterate over received values (blocking)
    pub fn iter(&mut self) -> impl Iterator<Item = T> + '_ {
        std::iter::from_fn(|| self.recv())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_send_recv() {
        let (tx, mut rx) = channel::<i32>(16);
        tx.send(42).unwrap();
        assert_eq!(rx.recv(), Some(42));
    }

    #[test]
    fn channel_try_send_full() {
        let (tx, _rx) = channel::<i32>(1);
        assert!(tx.try_send(1).is_ok());
        assert!(tx.try_send(2).is_err()); // Channel full
    }

    #[test]
    fn channel_recv_disconnected() {
        let (tx, mut rx) = channel::<i32>(16);
        tx.send(1).unwrap();
        drop(tx);
        assert_eq!(rx.recv(), Some(1));
        assert_eq!(rx.recv(), None); // Disconnected
    }
}
