//! # 9S Protocol
//!
//! Plan 9-inspired namespace abstraction. Five operations, never a sixth.
//!
//! Everything is a Scroll. Mount namespaces at paths. Operations route by longest prefix.
//!
//! ## The Five Operations
//!
//! ```text
//! read(path)         → Option<Scroll>     // Get data
//! write(path, data)  → Scroll             // Put data
//! list(prefix)       → Vec<String>        // Enumerate children
//! watch(pattern)     → Receiver<Scroll>   // Subscribe to changes
//! close()            → ()                 // Cleanup
//! ```
//!
//! ## Quick Start
//!
//! ```rust
//! use beewallet_core_spark::nine_s::{Kernel, MemoryNamespace, Namespace, Scroll};
//! use serde_json::json;
//!
//! // Create kernel with memory backend
//! let kernel = Kernel::new();
//! kernel.mount_box("/", MemoryNamespace::new());
//!
//! // Write and read
//! kernel.write("/hello", json!({"message": "world"})).unwrap();
//! let scroll = kernel.read("/hello").unwrap().unwrap();
//! assert_eq!(scroll.data["message"], "world");
//! ```
//!
//! ## Architecture
//!
//! - [`Scroll`]: Universal data envelope (key + JSON data + metadata)
//! - [`Namespace`]: The 5-operation trait any backend implements
//! - [`Kernel`]: Mount table for composing namespaces
//! - [`MemoryNamespace`]: In-memory backend with wildcard watch support
//!
//! ## WASM Compatibility
//!
//! The core types (Scroll, Namespace trait, Kernel) are WASM-compatible.
//! Enable `std-channel` feature for watch support with std::sync channels.

pub mod scroll;
pub mod namespace;
pub mod kernel;
pub mod backends;
pub mod store;
pub mod patch;
pub mod anchor;

// Sealed scrolls for sharing (requires crypto feature)
#[cfg(feature = "crypto")]
pub mod sealed;

#[cfg(feature = "std-channel")]
pub mod channel;

pub use scroll::{Scroll, Metadata, Tense, current_iso_time, current_time_millis};
pub use scroll::{types, kingdoms, verbs};
pub use namespace::{Namespace, Error, Result, Receiver};
pub use kernel::Kernel;
pub use store::Store;
pub use backends::memory::MemoryNamespace;

// Git-like primitives
pub use patch::{Patch, PatchOp, PatchError};
pub use anchor::Anchor;

// Sealed scrolls for sharing (requires crypto feature)
#[cfg(feature = "crypto")]
pub use sealed::{SealedScroll, MAX_SEALED_SIZE};

// FileNamespace is exposed for advanced use cases, but Store is preferred
pub use backends::file::FileNamespace;

#[cfg(feature = "std-channel")]
pub use backends::memory::WatchReceiver;
