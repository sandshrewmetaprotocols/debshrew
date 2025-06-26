//! In-memory adapters for testing debshrew
//!
//! This module provides in-memory implementations of the metashrew client traits
//! for fast testing and simulation. It follows the same patterns as metashrew's
//! memshrew adapter.

use crate::error::{Error, Result};
use crate::traits::{BlockProviderLike, ViewProviderLike, MetashrewClientLike, BlockchainSimulatorLike};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// In-memory metashrew client adapter for testing
///
/// This adapter provides a complete in-memory implementation of the metashrew
/// client interface, allowing for fast testing without requiring a real
/// metashrew service.
#[derive(Debug, Clone)]
pub struct MemoryMetashrewAdapter {
    /// Shared state between clones
    state: Arc<Mutex<AdapterState>>,
}

#[derive(Debug)]
struct AdapterState {
    /// Current block height
    height: u32,
    
    /// Block hashes by height
    block_hashes: HashMap<u32, Vec<u8>>,
    
    /// View function results: (view_name, params, height) -> result
    view_results: HashMap<(String, Vec<u8>, Option<u32>), Vec<u8>>,
    
    /// Client identifier
    identifier: String,
}

impl MemoryMetashrewAdapter {
    /// Create a new memory adapter
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(AdapterState {
                height: 0,
                block_hashes: HashMap::new(),
                view_results: HashMap::new(),
                identifier: "memory-adapter".to_string(),
            })),
        }
    }
    
    /// Create a new memory adapter with a custom identifier
    pub fn with_identifier(identifier: &str) -> Self {
        Self {
            state: Arc::new(Mutex::new(AdapterState {
                height: 0,
                block_hashes: HashMap::new(),
                view_results: HashMap::new(),
                identifier: identifier.to_string(),
            })),
        }
    }
    
    /// Set the current block height
    pub fn set_height(&self, height: u32) {
        let mut state = self.state.lock().unwrap();
        state.height = height;
    }
    
    /// Set a block hash for a given height
    pub fn set_block_hash(&self, height: u32, hash: Vec<u8>) {
        let mut state = self.state.lock().unwrap();
        state.block_hashes.insert(height, hash);
    }
    
    /// Set the result for a view function call
    pub fn set_view_result(&self, view_name: &str, params: &[u8], height: Option<u32>, result: Vec<u8>) {
        let mut state = self.state.lock().unwrap();
        let key = (view_name.to_string(), params.to_vec(), height);
        state.view_results.insert(key, result);
    }
    
    /// Clear all data
    pub fn clear(&self) {
        let mut state = self.state.lock().unwrap();
        state.height = 0;
        state.block_hashes.clear();
        state.view_results.clear();
    }
    
    /// Get the number of stored block hashes
    pub fn block_count(&self) -> usize {
        let state = self.state.lock().unwrap();
        state.block_hashes.len()
    }
    
    /// Get the number of stored view results
    pub fn view_result_count(&self) -> usize {
        let state = self.state.lock().unwrap();
        state.view_results.len()
    }
    
    /// Create a deep copy with isolated state
    pub fn deep_copy(&self) -> Self {
        let state = self.state.lock().unwrap();
        Self {
            state: Arc::new(Mutex::new(AdapterState {
                height: state.height,
                block_hashes: state.block_hashes.clone(),
                view_results: state.view_results.clone(),
                identifier: format!("{}-copy", state.identifier),
            })),
        }
    }
}

impl Default for MemoryMetashrewAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BlockProviderLike for MemoryMetashrewAdapter {
    async fn get_height(&self) -> Result<u32> {
        let state = self.state.lock().unwrap();
        Ok(state.height)
    }
    
    async fn get_block_hash(&self, height: u32) -> Result<Vec<u8>> {
        let state = self.state.lock().unwrap();
        state.block_hashes.get(&height)
            .cloned()
            .ok_or_else(|| Error::MetashrewClient(format!("Block hash not found for height {}", height)))
    }
}

#[async_trait]
impl ViewProviderLike for MemoryMetashrewAdapter {
    async fn call_view(&self, view_name: &str, params: &[u8], height: Option<u32>) -> Result<Vec<u8>> {
        let state = self.state.lock().unwrap();
        let key = (view_name.to_string(), params.to_vec(), height);
        
        state.view_results.get(&key)
            .cloned()
            .ok_or_else(|| Error::MetashrewClient(format!(
                "View result not found for view '{}' with {} bytes params at height {:?}",
                view_name, params.len(), height
            )))
    }
}

#[async_trait]
impl MetashrewClientLike for MemoryMetashrewAdapter {
    fn get_identifier(&self) -> String {
        let state = self.state.lock().unwrap();
        state.identifier.clone()
    }
    
    async fn is_healthy(&self) -> bool {
        true // Memory adapter is always healthy
    }
}

impl BlockchainSimulatorLike for MemoryMetashrewAdapter {
    fn advance_block(&mut self, block_data: Option<&[u8]>) -> Result<(u32, Vec<u8>)> {
        let mut state = self.state.lock().unwrap();
        let new_height = state.height + 1;
        
        // Generate a block hash based on height and optional data
        let hash = if let Some(data) = block_data {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            
            let mut hasher = DefaultHasher::new();
            new_height.hash(&mut hasher);
            data.hash(&mut hasher);
            let hash_value = hasher.finish();
            hash_value.to_be_bytes().to_vec()
        } else {
            // Simple deterministic hash based on height
            let mut hash = vec![0u8; 32];
            let height_bytes = new_height.to_be_bytes();
            hash[..4].copy_from_slice(&height_bytes);
            hash
        };
        
        state.height = new_height;
        state.block_hashes.insert(new_height, hash.clone());
        
        Ok((new_height, hash))
    }
    
    fn simulate_reorg(&mut self, fork_height: u32, new_blocks: Vec<Vec<u8>>) -> Result<(u32, Vec<u8>)> {
        let mut state = self.state.lock().unwrap();
        
        if fork_height > state.height {
            return Err(Error::MetashrewClient(format!(
                "Fork height {} is greater than current height {}",
                fork_height, state.height
            )));
        }
        
        // Remove blocks after fork height
        state.block_hashes.retain(|&height, _| height <= fork_height);
        
        // Add new blocks
        let mut current_height = fork_height;
        let mut last_hash = Vec::new();
        
        for (i, block_data) in new_blocks.iter().enumerate() {
            current_height += 1;
            
            // Generate hash for new block
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            
            let mut hasher = DefaultHasher::new();
            current_height.hash(&mut hasher);
            block_data.hash(&mut hasher);
            i.hash(&mut hasher); // Add index to ensure uniqueness
            let hash_value = hasher.finish();
            let hash = hash_value.to_be_bytes().to_vec();
            
            state.block_hashes.insert(current_height, hash.clone());
            last_hash = hash;
        }
        
        state.height = current_height;
        Ok((current_height, last_hash))
    }
    
    fn set_view_result(&mut self, view_name: &str, params: &[u8], height: Option<u32>, result: Vec<u8>) {
        let mut state = self.state.lock().unwrap();
        let key = (view_name.to_string(), params.to_vec(), height);
        state.view_results.insert(key, result);
    }
    
    fn get_view_results(&self) -> HashMap<(String, Vec<u8>, Option<u32>), Vec<u8>> {
        let state = self.state.lock().unwrap();
        state.view_results.clone()
    }
}

/// Builder for creating memory adapters with pre-configured data
pub struct MemoryAdapterBuilder {
    adapter: MemoryMetashrewAdapter,
}

impl MemoryAdapterBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            adapter: MemoryMetashrewAdapter::new(),
        }
    }
    
    /// Set the identifier
    pub fn with_identifier(mut self, identifier: &str) -> Self {
        self.adapter = MemoryMetashrewAdapter::with_identifier(identifier);
        self
    }
    
    /// Set the initial height
    pub fn with_height(self, height: u32) -> Self {
        self.adapter.set_height(height);
        self
    }
    
    /// Add a block hash
    pub fn with_block_hash(self, height: u32, hash: Vec<u8>) -> Self {
        self.adapter.set_block_hash(height, hash);
        self
    }
    
    /// Add a view result
    pub fn with_view_result(self, view_name: &str, params: &[u8], height: Option<u32>, result: Vec<u8>) -> Self {
        self.adapter.set_view_result(view_name, params, height, result);
        self
    }
    
    /// Build the adapter
    pub fn build(self) -> MemoryMetashrewAdapter {
        self.adapter
    }
}

impl Default for MemoryAdapterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_memory_adapter_basic_operations() {
        let adapter = MemoryMetashrewAdapter::new();
        
        // Test initial state
        assert_eq!(adapter.get_height().await.unwrap(), 0);
        assert!(adapter.get_block_hash(0).await.is_err());
        
        // Set height and block hash
        adapter.set_height(123);
        adapter.set_block_hash(123, vec![1, 2, 3, 4]);
        
        assert_eq!(adapter.get_height().await.unwrap(), 123);
        assert_eq!(adapter.get_block_hash(123).await.unwrap(), vec![1, 2, 3, 4]);
        
        // Test view function
        adapter.set_view_result("test_view", &[5, 6], Some(123), vec![7, 8, 9]);
        let result = adapter.call_view("test_view", &[5, 6], Some(123)).await.unwrap();
        assert_eq!(result, vec![7, 8, 9]);
        
        // Test missing view result
        assert!(adapter.call_view("missing_view", &[], None).await.is_err());
    }
    
    #[tokio::test]
    async fn test_memory_adapter_builder() {
        let adapter = MemoryAdapterBuilder::new()
            .with_identifier("test-adapter")
            .with_height(100)
            .with_block_hash(100, vec![0xaa, 0xbb])
            .with_view_result("test", &[1, 2], None, vec![3, 4])
            .build();
        
        assert_eq!(adapter.get_identifier(), "test-adapter");
        assert_eq!(adapter.get_height().await.unwrap(), 100);
        assert_eq!(adapter.get_block_hash(100).await.unwrap(), vec![0xaa, 0xbb]);
        assert_eq!(adapter.call_view("test", &[1, 2], None).await.unwrap(), vec![3, 4]);
    }
    
    #[test]
    fn test_blockchain_simulator() {
        let mut adapter = MemoryMetashrewAdapter::new();
        
        // Test advancing blocks
        let (height1, hash1) = adapter.advance_block(Some(b"block1")).unwrap();
        assert_eq!(height1, 1);
        assert!(!hash1.is_empty());
        
        let (height2, hash2) = adapter.advance_block(Some(b"block2")).unwrap();
        assert_eq!(height2, 2);
        assert_ne!(hash1, hash2);
        
        // Test reorg
        let new_blocks = vec![b"reorg_block1".to_vec(), b"reorg_block2".to_vec()];
        let (new_height, new_hash) = adapter.simulate_reorg(1, new_blocks).unwrap();
        assert_eq!(new_height, 3);
        assert_ne!(new_hash, hash2);
        
        // Verify the reorg worked
        assert_eq!(adapter.block_count(), 3); // Genesis + 2 reorg blocks
    }
    
    #[test]
    fn test_deep_copy() {
        let adapter1 = MemoryMetashrewAdapter::new();
        adapter1.set_height(42);
        adapter1.set_block_hash(42, vec![1, 2, 3]);
        
        let adapter2 = adapter1.deep_copy();
        
        // Both should have the same data initially
        assert_eq!(
            futures::executor::block_on(adapter1.get_height()).unwrap(),
            futures::executor::block_on(adapter2.get_height()).unwrap()
        );
        
        // Modifying one shouldn't affect the other
        adapter1.set_height(100);
        assert_eq!(futures::executor::block_on(adapter1.get_height()).unwrap(), 100);
        assert_eq!(futures::executor::block_on(adapter2.get_height()).unwrap(), 42);
    }
}