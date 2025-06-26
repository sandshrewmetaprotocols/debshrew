//! Core traits for generic metashrew client backends
//!
//! This module defines the fundamental traits that enable Debshrew to work with
//! different metashrew backends in a generic way. The primary traits are:
//!
//! - [`MetashrewClientLike`]: The main client interface that abstracts metashrew operations
//! - [`BlockProviderLike`]: Interface for providing block data and metadata
//! - [`ViewProviderLike`]: Interface for executing view functions
//!
//! These traits enable dependency injection and allow Debshrew to work with
//! real metashrew services, in-memory mocks, or other implementations without changing
//! the core runtime logic.
//!
//! # Architecture
//!
//! The client layer follows a generic design pattern where the runtime is
//! parameterized over a client type `T: MetashrewClientLike`. This allows:
//!
//! - **Testing**: Use in-memory mocks for fast unit tests
//! - **Production**: Use real metashrew JSON-RPC clients
//! - **Future extensibility**: Add support for other metashrew interfaces

use crate::error::Result;
use async_trait::async_trait;
use std::fmt::Debug;

/// Trait for providing block data and metadata
///
/// This trait defines the interface for accessing Bitcoin block information
/// from a metashrew service or mock implementation.
#[async_trait]
pub trait BlockProviderLike: Send + Sync + Debug {
    /// Get the current block height
    ///
    /// # Returns
    ///
    /// The current block height
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails
    async fn get_height(&self) -> Result<u32>;
    
    /// Get the block hash for a given height
    ///
    /// # Arguments
    ///
    /// * `height` - The block height
    ///
    /// # Returns
    ///
    /// The block hash as bytes
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or block doesn't exist
    async fn get_block_hash(&self, height: u32) -> Result<Vec<u8>>;
}

/// Trait for executing view functions
///
/// This trait defines the interface for calling metashrew view functions
/// with optional height-based historical queries.
#[async_trait]
pub trait ViewProviderLike: Send + Sync + Debug {
    /// Call a view function
    ///
    /// # Arguments
    ///
    /// * `view_name` - The name of the view function
    /// * `params` - The parameters to pass to the view function
    /// * `height` - The block height to query at (optional)
    ///
    /// # Returns
    ///
    /// The result of the view function as bytes
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails
    async fn call_view(&self, view_name: &str, params: &[u8], height: Option<u32>) -> Result<Vec<u8>>;
}

/// Generic trait for metashrew client backends
///
/// This is the core client abstraction that allows Debshrew to work with
/// different metashrew implementations. It combines block and view provider
/// functionality into a single interface.
///
/// # Design Principles
///
/// - **Generic**: Works with any metashrew service that implements this trait
/// - **Async**: Supports asynchronous operations for network clients
/// - **Historical**: Supports height-based historical queries
/// - **Extensible**: Can be extended with additional metashrew features
///
/// # Implementation Notes
///
/// Implementors should ensure that:
/// - Operations are thread-safe
/// - Historical queries are consistent
/// - Error types provide meaningful debugging information
/// - Network timeouts and retries are handled appropriately
#[async_trait]
pub trait MetashrewClientLike: BlockProviderLike + ViewProviderLike + Send + Sync + Debug {
    /// Get a human-readable identifier for this client
    ///
    /// This is used for logging and debugging purposes.
    ///
    /// # Returns
    ///
    /// A string identifier for the client (e.g., URL, "mock", etc.)
    fn get_identifier(&self) -> String;
    
    /// Check if the client is healthy and operational
    ///
    /// This can be used for health checks and connection management.
    /// The default implementation assumes the client is always healthy.
    ///
    /// # Returns
    ///
    /// `true` if the client is operational, `false` otherwise.
    async fn is_healthy(&self) -> bool {
        true // Default implementation assumes always healthy
    }
}

/// Trait for blockchain simulation in tests
///
/// This trait provides additional functionality needed for testing,
/// such as simulating block progression and reorgs.
pub trait BlockchainSimulatorLike: MetashrewClientLike {
    /// Advance the blockchain by one block
    ///
    /// # Arguments
    ///
    /// * `block_data` - Optional block data to use for the new block
    ///
    /// # Returns
    ///
    /// The new block height and hash
    ///
    /// # Errors
    ///
    /// Returns an error if the simulation fails
    fn advance_block(&mut self, block_data: Option<&[u8]>) -> Result<(u32, Vec<u8>)>;
    
    /// Simulate a chain reorganization
    ///
    /// # Arguments
    ///
    /// * `fork_height` - The height at which the fork occurs
    /// * `new_blocks` - The new blocks to replace the old ones
    ///
    /// # Returns
    ///
    /// The new tip height and hash
    ///
    /// # Errors
    ///
    /// Returns an error if the reorg simulation fails
    fn simulate_reorg(&mut self, fork_height: u32, new_blocks: Vec<Vec<u8>>) -> Result<(u32, Vec<u8>)>;
    
    /// Set the result for a view function call
    ///
    /// # Arguments
    ///
    /// * `view_name` - The name of the view function
    /// * `params` - The parameters to match
    /// * `height` - The block height to match (optional)
    /// * `result` - The result to return
    fn set_view_result(&mut self, view_name: &str, params: &[u8], height: Option<u32>, result: Vec<u8>);
    
    /// Get the current state of all view results
    ///
    /// # Returns
    ///
    /// A map of view function calls to their results
    fn get_view_results(&self) -> std::collections::HashMap<(String, Vec<u8>, Option<u32>), Vec<u8>>;
}

/// Result of atomic block processing for testing
///
/// This structure encapsulates the complete result of processing a Bitcoin block
/// atomically, including CDC messages, state changes, and metadata.
#[derive(Debug, Clone)]
pub struct AtomicBlockResult {
    /// The CDC messages generated during block processing
    pub cdc_messages: Vec<crate::CdcMessage>,
    
    /// The transform state after processing
    pub transform_state: crate::TransformState,
    
    /// The block height that was processed
    pub height: u32,
    
    /// The block hash
    pub block_hash: Vec<u8>,
    
    /// Processing timestamp
    pub timestamp: u64,
}

impl AtomicBlockResult {
    /// Create a new atomic block result
    pub fn new(
        cdc_messages: Vec<crate::CdcMessage>,
        transform_state: crate::TransformState,
        height: u32,
        block_hash: Vec<u8>,
        timestamp: u64,
    ) -> Self {
        Self {
            cdc_messages,
            transform_state,
            height,
            block_hash,
            timestamp,
        }
    }
}