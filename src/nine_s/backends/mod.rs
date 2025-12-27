//! 9S Namespace Backends
//!
//! - Memory: In-memory (testing, transient state)
//! - File: Persistent local storage (JSON files)

pub mod memory;
pub mod file;

pub use memory::MemoryNamespace;
pub use file::FileNamespace;
