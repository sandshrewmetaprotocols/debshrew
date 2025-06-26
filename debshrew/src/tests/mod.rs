//! Comprehensive test suite for debshrew with maximum e2e coverage
//!
//! This module provides comprehensive end-to-end testing for the debshrew framework
//! using in-memory adapters and the debshrew-minimal WASM module.
//! 
//! The test suite is optimized for:
//! - Maximum code coverage with minimal test count
//! - Fast execution using in-memory adapters
//! - Complete e2e validation including debshrew-minimal WASM execution
//! - Real-world scenarios including chain reorganizations and CDC generation
//! - Integration with mock metashrew services

use crate::error::Result;
use crate::traits::{MetashrewClientLike, BlockchainSimulatorLike, BlockProviderLike, ViewProviderLike};
use crate::adapters::MemoryMetashrewAdapter;
use crate::{CdcMessage, CdcHeader, CdcOperation, CdcPayload, TransformState};
use std::path::PathBuf;

// Core test modules
pub mod block_builder;
pub mod comprehensive_e2e_test;
pub mod integration_tests;
pub mod reorg_focused_test;

/// Test configuration and utilities
pub struct TestConfig {
    pub wasm_path: PathBuf,
}

impl TestConfig {
    pub fn new() -> Self {
        let wasm_path = if let Ok(path) = std::env::var("DEBSHREW_MINIMAL_WASM") {
            PathBuf::from(path)
        } else {
            PathBuf::from("./target/wasm32-unknown-unknown/release/debshrew_minimal.wasm")
        };
        Self { wasm_path }
    }

    /// Create a new runtime instance for testing
    pub fn create_runtime(&self) -> Result<crate::runtime::WasmRuntime> {
        crate::runtime::WasmRuntime::new(&self.wasm_path, "http://localhost:18888")
    }
    
    /// Create a memory adapter with metashrew-minimal view functions
    pub fn create_metashrew_adapter(&self) -> MemoryMetashrewAdapter {
        let adapter = MemoryMetashrewAdapter::with_identifier("test-metashrew");
        
        // Set up basic metashrew-minimal view functions
        // These simulate the behavior of metashrew-minimal's blocktracker and getblock views
        
        adapter
    }
}

/// Test utilities for creating Bitcoin-like block data and CDC scenarios
pub struct TestUtils;

impl TestUtils {
    /// Create test block data with specified content
    pub fn create_test_block_data(height: u32, content: &[u8]) -> Vec<u8> {
        let mut block_data = Vec::new();
        
        // Add height as first 4 bytes
        block_data.extend_from_slice(&height.to_le_bytes());
        
        // Add content
        block_data.extend_from_slice(content);
        
        // Pad to minimum size if needed
        while block_data.len() < 32 {
            block_data.push(0);
        }
        
        block_data
    }
    
    /// Create a sequence of test blocks
    pub fn create_test_blocks(count: u32, prefix: &str) -> Vec<Vec<u8>> {
        (0..count)
            .map(|i| {
                let content = format!("{}_block_{}", prefix, i);
                Self::create_test_block_data(i, content.as_bytes())
            })
            .collect()
    }
    
    /// Set up metashrew-minimal view functions on an adapter
    pub fn setup_metashrew_minimal_views(adapter: &MemoryMetashrewAdapter, blocks: &[Vec<u8>]) {
        // Set up blocktracker view function
        // The blocktracker should return the first byte of each block hash
        for height in 0..blocks.len() {
            let mut blocktracker_data = Vec::new();
            
            // For each height from 0 to current height, add the first byte of the block hash
            for h in 0..=height {
                if let Some(block_data) = blocks.get(h) {
                    // Use a simple hash of the block data
                    let hash_byte = Self::simple_hash(block_data);
                    blocktracker_data.push(hash_byte);
                }
            }
            
            // Set the blocktracker result for this height
            adapter.set_view_result("blocktracker", &[], Some(height as u32), blocktracker_data);
        }
        
        // Set up getblock view function
        for (height, block_data) in blocks.iter().enumerate() {
            let height_input = (height as u32).to_le_bytes().to_vec();
            adapter.set_view_result("getblock", &height_input, Some(height as u32), block_data.clone());
        }
    }
    
    /// Simple hash function for test block data
    fn simple_hash(data: &[u8]) -> u8 {
        data.iter().fold(0u8, |acc, &b| acc.wrapping_add(b))
    }
    
    /// Create expected CDC messages for a block sequence
    pub fn create_expected_cdc_messages(blocks: &[Vec<u8>]) -> Vec<crate::CdcMessage> {
        let mut messages = Vec::new();
        
        for (height, block_data) in blocks.iter().enumerate() {
            let hash_byte = Self::simple_hash(block_data);
            
            let message = crate::CdcMessage {
                header: crate::CdcHeader {
                    source: "debshrew-minimal".to_string(),
                    timestamp: 0, // We'll ignore timestamp in comparisons
                    block_height: height as u32,
                    block_hash: format!("{:02x}", hash_byte),
                    transaction_id: None,
                },
                payload: crate::CdcPayload {
                    operation: crate::CdcOperation::Create,
                    table: "blocks".to_string(),
                    key: height.to_string(),
                    before: None,
                    after: Some(serde_json::json!({
                        "height": height as u32,
                        "hash_byte": hash_byte,
                        "tracker_length": height + 1
                    })),
                },
            };
            
            messages.push(message);
        }
        
        messages
    }
    
    /// Compare CDC messages ignoring timestamps
    pub fn compare_cdc_messages_ignore_timestamp(
        actual: &[crate::CdcMessage],
        expected: &[crate::CdcMessage],
    ) -> bool {
        if actual.len() != expected.len() {
            return false;
        }
        
        for (a, e) in actual.iter().zip(expected.iter()) {
            if a.header.source != e.header.source
                || a.header.block_height != e.header.block_height
                || a.header.block_hash != e.header.block_hash
                || a.header.transaction_id != e.header.transaction_id
                || a.payload != e.payload
            {
                return false;
            }
        }
        
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = TestConfig::new();
        // We can't test if the WASM file exists since it might not be built yet
        // But we can test that the path is set correctly
        assert!(config.wasm_path.to_string_lossy().contains("debshrew_minimal.wasm"));
    }

    #[test]
    fn test_test_utils() {
        let block_data = TestUtils::create_test_block_data(5, b"test");
        assert!(block_data.len() >= 32);
        assert_eq!(&block_data[0..4], &5u32.to_le_bytes());
        
        let blocks = TestUtils::create_test_blocks(3, "test");
        assert_eq!(blocks.len(), 3);
        
        let hash1 = TestUtils::simple_hash(&blocks[0]);
        let hash2 = TestUtils::simple_hash(&blocks[1]);
        assert_ne!(hash1, hash2); // Different blocks should have different hashes
    }

    #[tokio::test]
    async fn test_memory_adapter_setup() {
        let adapter = MemoryMetashrewAdapter::new();
        let blocks = TestUtils::create_test_blocks(3, "test");
        
        TestUtils::setup_metashrew_minimal_views(&adapter, &blocks);
        
        // Test blocktracker view
        let blocktracker_0 = adapter.call_view("blocktracker", &[], Some(0)).await.unwrap();
        assert_eq!(blocktracker_0.len(), 1);
        
        let blocktracker_2 = adapter.call_view("blocktracker", &[], Some(2)).await.unwrap();
        assert_eq!(blocktracker_2.len(), 3);
        
        // Test getblock view
        let height_input = 1u32.to_le_bytes().to_vec();
        let block_1 = adapter.call_view("getblock", &height_input, Some(1)).await.unwrap();
        assert_eq!(block_1, blocks[1]);
    }

    #[test]
    fn test_cdc_message_comparison() {
        let blocks = TestUtils::create_test_blocks(2, "test");
        let messages = TestUtils::create_expected_cdc_messages(&blocks);
        
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].header.block_height, 0);
        assert_eq!(messages[1].header.block_height, 1);
        assert_eq!(messages[0].payload.table, "blocks");
        assert_eq!(messages[1].payload.table, "blocks");
        
        // Test comparison function
        assert!(TestUtils::compare_cdc_messages_ignore_timestamp(&messages, &messages));
        
        let mut modified_messages = messages.clone();
        modified_messages[0].header.timestamp = 12345; // Change timestamp
        assert!(TestUtils::compare_cdc_messages_ignore_timestamp(&messages, &modified_messages));
        
        modified_messages[0].header.block_height = 999; // Change something else
        assert!(!TestUtils::compare_cdc_messages_ignore_timestamp(&messages, &modified_messages));
    }
}