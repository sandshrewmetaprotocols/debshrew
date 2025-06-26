//! A framework for building deterministic CDC streams from Bitcoin metaprotocol state
//!
//! Debshrew enables building highly available, reorg-aware ETL pipelines that transform
//! metaprotocol state into standardized CDC streams consumable by the debezium ecosystem.
//!
//! # Core Features
//!
//! - WASM-based transformation programs
//! - Deterministic CDC generation from metashrew views
//! - Automatic reorg handling with state rollbacks
//! - 6-block caching for reorg protection
//! - Debezium-compatible CDC output
//! - Extensible metaprotocol support
//!
//! # Example
//!
//! ```no_run
//! use debshrew::{BlockSynchronizer, MetashrewClient, JsonRpcClient, create_sink, SinkConfig, WasmRuntime};
//! use std::path::Path;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Create a metashrew client
//!     let client = JsonRpcClient::new("http://localhost:8080")?;
//!
//!     // Load the transform module
//!     let runtime = WasmRuntime::new(Path::new("transform.wasm"), "http://localhost:8080")?;
//!
//!     // Create a CDC sink
//!     let sink_config = SinkConfig::Kafka {
//!         bootstrap_servers: "localhost:9092".to_string(),
//!         topic: "cdc-events".to_string(),
//!         client_id: None,
//!         batch_size: 100,
//!         flush_interval: 1000,
//!     };
//!     let sink = create_sink(&sink_config)?;
//!
//!     // Create a block synchronizer
//!     let mut synchronizer = BlockSynchronizer::new(client, runtime, sink, 6)?;
//!
//!     // Start synchronization
//!     synchronizer.run().await?;
//!
//!     Ok(())
//! }
//! ```

#![warn(missing_docs)]
#![warn(rustdoc::missing_doc_code_examples)]

pub mod adapters;
pub mod block;
pub mod client;
pub mod config;
pub mod error;
pub mod runtime;
pub mod sink;
pub mod synchronizer;
pub mod traits;

#[cfg(test)]
pub mod tests;

/// Re-export common types and functions for convenience
pub use block::BlockCache;
pub use client::*;
pub use config::*;
pub use runtime::WasmRuntime;
pub use debshrew_support::{CdcMessage, CdcHeader, CdcOperation, CdcPayload, TransformState};
pub use error::{Error, Result};
pub use sink::{CdcSink, create_sink, ConsoleSink, FileSink, KafkaSink, NullSink, PostgresSink};
pub use synchronizer::{BlockSynchronizer, Synchronizer};
pub use traits::*;