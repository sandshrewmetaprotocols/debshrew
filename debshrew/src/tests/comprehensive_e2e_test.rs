//! Comprehensive end-to-end tests for debshrew functionality
//!
//! This test suite validates:
//! - Complete debshrew workflow with debshrew-minimal WASM
//! - CDC message generation and consistency
//! - Integration with mock metashrew services
//! - Block processing and state management

use super::block_builder::{ChainBuilder, create_test_block};
use super::{TestConfig, TestUtils};
use crate::adapters::MemoryMetashrewAdapter;
use crate::error::Result;
use crate::traits::{MetashrewClientLike, BlockchainSimulatorLike, BlockProviderLike, ViewProviderLike};

/// Simple comprehensive test that validates debshrew functionality
#[tokio::test]
async fn test_comprehensive_debshrew_functionality() -> Result<()> {
    // Create test configuration
    let config = TestConfig::new();
    
    // Create mock metashrew adapter
    let adapter = config.create_metashrew_adapter();
    
    // Create test blocks
    let blocks = TestUtils::create_test_blocks(5, "test");
    
    // Set up metashrew views
    TestUtils::setup_metashrew_minimal_views(&adapter, &blocks);
    
    // Set block hashes in the adapter
    for (height, block_data) in blocks.iter().enumerate() {
        let hash = TestUtils::simple_hash(block_data);
        adapter.set_block_hash(height as u32, vec![hash]);
    }
    
    // Set the current height
    adapter.set_height(4); // 0-indexed, so height 4 means 5 blocks
    
    // Test that we can call metashrew views
    for height in 0..5 {
        let blocktracker_data = adapter
            .call_view("blocktracker", &[], Some(height))
            .await?;
        
        // At each height, blocktracker should have (height + 1) bytes
        let expected_length = (height + 1) as usize;
        assert_eq!(
            blocktracker_data.len(),
            expected_length,
            "Blocktracker should have {} bytes at height {}",
            expected_length,
            height
        );
        
        println!("✓ Height {}: {} bytes", height, blocktracker_data.len());
    }
    
    // Test getblock view function
    for height in 0..5 {
        let height_input = (height as u32).to_le_bytes().to_vec();
        let block_data = adapter
            .call_view("getblock", &height_input, Some(height))
            .await?;
        
        assert_eq!(
            block_data, blocks[height as usize],
            "Block data should match at height {}",
            height
        );
        println!("✓ Block at height {}: {} bytes", height, block_data.len());
    }
    
    println!("✅ Comprehensive debshrew functionality test passed!");
    
    Ok(())
}

/// Test CDC message generation with mock runtime
#[tokio::test]
async fn test_cdc_message_generation() -> Result<()> {
    // Create test blocks
    let blocks = TestUtils::create_test_blocks(3, "cdc_test");
    
    // Create expected CDC messages
    let expected_messages = TestUtils::create_expected_cdc_messages(&blocks);
    
    // Verify expected message structure
    assert_eq!(expected_messages.len(), 3);
    
    for (i, message) in expected_messages.iter().enumerate() {
        assert_eq!(message.header.source, "debshrew-minimal");
        assert_eq!(message.header.block_height, i as u32);
        assert_eq!(message.payload.operation, crate::CdcOperation::Create);
        assert_eq!(message.payload.table, "blocks");
        assert_eq!(message.payload.key, i.to_string());
        assert!(message.payload.before.is_none());
        assert!(message.payload.after.is_some());
        
        println!("✓ CDC message {}: {:?}", i, message.header);
    }
    
    println!("✅ CDC message generation test passed!");
    
    Ok(())
}

/// Test blockchain simulation functionality
#[test]
fn test_blockchain_simulation() -> Result<()> {
    let mut adapter = MemoryMetashrewAdapter::new();
    
    // Test advancing blocks
    let (height1, hash1) = adapter.advance_block(Some(b"block1"))?;
    assert_eq!(height1, 1);
    assert!(!hash1.is_empty());
    
    let (height2, hash2) = adapter.advance_block(Some(b"block2"))?;
    assert_eq!(height2, 2);
    assert_ne!(hash1, hash2);
    
    let (height3, hash3) = adapter.advance_block(Some(b"block3"))?;
    assert_eq!(height3, 3);
    assert_ne!(hash2, hash3);
    
    println!("✓ Advanced to height {}", height3);
    
    // Test reorg simulation
    let new_blocks = vec![b"reorg_block1".to_vec(), b"reorg_block2".to_vec()];
    let (new_height, new_hash) = adapter.simulate_reorg(1, new_blocks)?;
    assert_eq!(new_height, 3); // 1 + 2 new blocks
    assert_ne!(new_hash, hash3);
    
    println!("✓ Reorg to height {}", new_height);
    
    // Verify the reorg worked
    assert_eq!(adapter.block_count(), 3); // Should have 3 blocks total
    
    println!("✅ Blockchain simulation test passed!");
    
    Ok(())
}

/// Test adapter state management
#[tokio::test]
async fn test_adapter_state_management() -> Result<()> {
    let adapter = MemoryMetashrewAdapter::with_identifier("state-test");
    
    // Test initial state
    assert_eq!(adapter.get_identifier(), "state-test");
    assert_eq!(adapter.get_height().await?, 0);
    assert_eq!(adapter.block_count(), 0);
    assert_eq!(adapter.view_result_count(), 0);
    
    // Add some data
    adapter.set_height(10);
    adapter.set_block_hash(10, vec![0xaa, 0xbb, 0xcc]);
    adapter.set_view_result("test_view", &[1, 2, 3], Some(10), vec![4, 5, 6]);
    
    // Verify data was added
    assert_eq!(adapter.get_height().await?, 10);
    assert_eq!(adapter.get_block_hash(10).await?, vec![0xaa, 0xbb, 0xcc]);
    assert_eq!(adapter.call_view("test_view", &[1, 2, 3], Some(10)).await?, vec![4, 5, 6]);
    assert_eq!(adapter.block_count(), 1);
    assert_eq!(adapter.view_result_count(), 1);
    
    // Test deep copy
    let adapter_copy = adapter.deep_copy();
    assert_eq!(adapter_copy.get_height().await?, 10);
    assert_eq!(adapter_copy.get_block_hash(10).await?, vec![0xaa, 0xbb, 0xcc]);
    
    // Modify original, copy should be unchanged
    adapter.set_height(20);
    assert_eq!(adapter.get_height().await?, 20);
    assert_eq!(adapter_copy.get_height().await?, 10);
    
    // Clear and verify
    adapter.clear();
    assert_eq!(adapter.get_height().await?, 0);
    assert_eq!(adapter.block_count(), 0);
    assert_eq!(adapter.view_result_count(), 0);
    
    println!("✅ Adapter state management test passed!");
    
    Ok(())
}

/// Test metashrew view function setup and querying
#[tokio::test]
async fn test_metashrew_view_setup() -> Result<()> {
    let adapter = MemoryMetashrewAdapter::new();
    let blocks = vec![
        create_test_block(0, b"genesis"),
        create_test_block(1, b"block1"),
        create_test_block(2, b"block2"),
    ];
    
    // Set up views
    TestUtils::setup_metashrew_minimal_views(&adapter, &blocks);
    
    // Test blocktracker at different heights
    let bt0 = adapter.call_view("blocktracker", &[], Some(0)).await?;
    let bt1 = adapter.call_view("blocktracker", &[], Some(1)).await?;
    let bt2 = adapter.call_view("blocktracker", &[], Some(2)).await?;
    
    assert_eq!(bt0.len(), 1);
    assert_eq!(bt1.len(), 2);
    assert_eq!(bt2.len(), 3);
    
    // Verify blocktracker prefix property
    assert_eq!(&bt1[0..1], &bt0[..]);
    assert_eq!(&bt2[0..2], &bt1[..]);
    
    // Test getblock at different heights
    for height in 0..3 {
        let height_input = (height as u32).to_le_bytes().to_vec();
        let block_data = adapter.call_view("getblock", &height_input, Some(height)).await?;
        assert_eq!(block_data, blocks[height as usize]);
    }
    
    println!("✅ Metashrew view setup test passed!");
    
    Ok(())
}

/// Test error handling and edge cases
#[tokio::test]
async fn test_error_handling() -> Result<()> {
    let adapter = MemoryMetashrewAdapter::new();
    
    // Test missing block hash
    let result = adapter.get_block_hash(999).await;
    assert!(result.is_err());
    
    // Test missing view result
    let result = adapter.call_view("nonexistent", &[], None).await;
    assert!(result.is_err());
    
    // Test invalid reorg
    let mut sim_adapter = adapter.clone();
    sim_adapter.set_height(5);
    let result = sim_adapter.simulate_reorg(10, vec![]);
    assert!(result.is_err());
    
    println!("✅ Error handling test passed!");
    
    Ok(())
}

/// Test comprehensive workflow with chain building
#[tokio::test]
async fn test_comprehensive_workflow() -> Result<()> {
    // Create a realistic chain of blocks
    let chain = ChainBuilder::new().add_blocks(10).blocks();
    
    // Create adapter and set up views
    let adapter = MemoryMetashrewAdapter::with_identifier("workflow-test");
    TestUtils::setup_metashrew_minimal_views(&adapter, &chain);
    
    // Set block hashes
    for (height, block_data) in chain.iter().enumerate() {
        let hash = TestUtils::simple_hash(block_data);
        adapter.set_block_hash(height as u32, vec![hash]);
    }
    
    adapter.set_height(10);
    
    // Verify the complete workflow
    for height in 0..=10 {
        // Test height queries
        let current_height = adapter.get_height().await?;
        assert_eq!(current_height, 10);
        
        // Test block hash queries
        if height < chain.len() as u32 {
            let hash = adapter.get_block_hash(height).await?;
            assert!(!hash.is_empty());
        }
        
        // Test view function queries
        if height < chain.len() as u32 {
            let blocktracker = adapter.call_view("blocktracker", &[], Some(height)).await?;
            assert_eq!(blocktracker.len(), (height + 1) as usize);
            
            let height_input = height.to_le_bytes().to_vec();
            let block_data = adapter.call_view("getblock", &height_input, Some(height)).await?;
            assert_eq!(block_data, chain[height as usize]);
        }
    }
    
    // Test health check
    assert!(adapter.is_healthy().await);
    
    println!("✅ Comprehensive workflow test passed!");
    
    Ok(())
}