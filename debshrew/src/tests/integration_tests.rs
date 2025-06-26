//! Integration tests for the complete debshrew workflow
//!
//! These tests verify end-to-end functionality including block processing,
//! CDC message generation, and integration with mock metashrew services.

use super::block_builder::ChainBuilder;
use super::{TestConfig, TestUtils};
use crate::adapters::MemoryMetashrewAdapter;
use crate::error::Result;
use crate::traits::{MetashrewClientLike, BlockchainSimulatorLike, BlockProviderLike, ViewProviderLike};
use crate::{CdcMessage, CdcOperation};

/// Test complete debshrew workflow with mock metashrew
#[tokio::test]
async fn test_complete_debshrew_workflow() -> Result<()> {
    // Create a realistic chain of blocks
    let chain = ChainBuilder::new().add_blocks(10).blocks();

    // Create adapter and set up metashrew views
    let adapter = MemoryMetashrewAdapter::with_identifier("integration-test");
    TestUtils::setup_metashrew_minimal_views(&adapter, &chain);

    // Set block hashes
    for (height, block_data) in chain.iter().enumerate() {
        let hash = TestUtils::simple_hash(block_data);
        adapter.set_block_hash(height as u32, vec![hash]);
    }

    adapter.set_height(10);

    // Verify complete workflow
    for height in 0..=10 {
        // Test metashrew client functionality
        let current_height = adapter.get_height().await?;
        assert_eq!(current_height, 10);

        // Test block queries
        if height < chain.len() as u32 {
            let hash = adapter.get_block_hash(height).await?;
            assert!(!hash.is_empty());

            // Test view functions
            let blocktracker = adapter.call_view("blocktracker", &[], Some(height)).await?;
            assert_eq!(blocktracker.len(), (height + 1) as usize);

            let height_input = height.to_le_bytes().to_vec();
            let block_data = adapter.call_view("getblock", &height_input, Some(height)).await?;
            assert_eq!(block_data, chain[height as usize]);
        }
    }

    // Test health check
    assert!(adapter.is_healthy().await);

    println!("✅ Complete debshrew workflow test passed!");
    Ok(())
}

/// Test CDC message generation and validation
#[tokio::test]
async fn test_cdc_message_generation_and_validation() -> Result<()> {
    // Create test blocks
    let blocks = TestUtils::create_test_blocks(5, "cdc_test");
    
    // Generate expected CDC messages
    let expected_messages = TestUtils::create_expected_cdc_messages(&blocks);
    
    // Validate message structure and content
    assert_eq!(expected_messages.len(), 5);
    
    for (i, message) in expected_messages.iter().enumerate() {
        // Validate header
        assert_eq!(message.header.source, "debshrew-minimal");
        assert_eq!(message.header.block_height, i as u32);
        assert!(!message.header.block_hash.is_empty());
        assert!(message.header.transaction_id.is_none());
        
        // Validate payload
        assert_eq!(message.payload.operation, CdcOperation::Create);
        assert_eq!(message.payload.table, "blocks");
        assert_eq!(message.payload.key, i.to_string());
        assert!(message.payload.before.is_none());
        assert!(message.payload.after.is_some());
        
        // Validate payload content
        if let Some(after) = &message.payload.after {
            assert_eq!(after["height"], i as u32);
            assert!(after["hash_byte"].is_number());
            assert_eq!(after["tracker_length"], i + 1);
        }
        
        println!("✓ CDC message {} validated", i);
    }
    
    // Test message comparison utility
    let messages_copy = expected_messages.clone();
    assert!(TestUtils::compare_cdc_messages_ignore_timestamp(&expected_messages, &messages_copy));
    
    // Test with modified timestamp (should still match)
    let mut modified_messages = expected_messages.clone();
    modified_messages[0].header.timestamp = 999999;
    assert!(TestUtils::compare_cdc_messages_ignore_timestamp(&expected_messages, &modified_messages));
    
    // Test with modified content (should not match)
    modified_messages[0].header.block_height = 999;
    assert!(!TestUtils::compare_cdc_messages_ignore_timestamp(&expected_messages, &modified_messages));
    
    println!("✅ CDC message generation and validation test passed!");
    Ok(())
}

/// Test blockchain simulation and state management
#[test]
fn test_blockchain_simulation_and_state_management() -> Result<()> {
    let mut adapter = MemoryMetashrewAdapter::with_identifier("simulation-test");
    
    // Test initial state
    assert_eq!(adapter.block_count(), 0);
    assert_eq!(adapter.view_result_count(), 0);
    
    // Test advancing blocks
    let mut block_hashes = Vec::new();
    for i in 0..5 {
        let block_data = format!("block_{}", i);
        let (height, hash) = adapter.advance_block(Some(block_data.as_bytes()))?;
        assert_eq!(height, i + 1);
        assert!(!hash.is_empty());
        block_hashes.push(hash);
    }
    
    assert_eq!(adapter.block_count(), 5);
    
    // Verify all blocks are accessible
    for i in 1..=5 {
        let hash = futures::executor::block_on(adapter.get_block_hash(i))?;
        assert_eq!(hash, block_hashes[(i - 1) as usize]);
    }
    
    // Test reorg simulation
    let original_height = futures::executor::block_on(adapter.get_height())?;
    assert_eq!(original_height, 5);
    
    let reorg_blocks = vec![
        b"reorg_block_1".to_vec(),
        b"reorg_block_2".to_vec(),
        b"reorg_block_3".to_vec(),
    ];
    
    let (new_height, new_hash) = adapter.simulate_reorg(2, reorg_blocks)?;
    assert_eq!(new_height, 5); // 2 + 3 new blocks
    assert_ne!(new_hash, block_hashes[4]); // Should be different from original
    
    // Verify reorg worked
    assert_eq!(adapter.block_count(), 5);
    let final_height = futures::executor::block_on(adapter.get_height())?;
    assert_eq!(final_height, 5);
    
    // Blocks 1-2 should be unchanged, 3-5 should be different
    for i in 1..=2 {
        let hash = futures::executor::block_on(adapter.get_block_hash(i))?;
        assert_eq!(hash, block_hashes[(i - 1) as usize]);
    }
    
    for i in 3..=5 {
        let hash = futures::executor::block_on(adapter.get_block_hash(i))?;
        assert_ne!(hash, block_hashes[(i - 1) as usize]);
    }
    
    println!("✅ Blockchain simulation and state management test passed!");
    Ok(())
}

/// Test metashrew view function integration
#[tokio::test]
async fn test_metashrew_view_function_integration() -> Result<()> {
    let adapter = MemoryMetashrewAdapter::new();
    
    // Create test blocks with specific content
    let blocks = vec![
        TestUtils::create_test_block_data(0, b"genesis"),
        TestUtils::create_test_block_data(1, b"block_one"),
        TestUtils::create_test_block_data(2, b"block_two"),
        TestUtils::create_test_block_data(3, b"block_three"),
    ];
    
    // Set up metashrew views
    TestUtils::setup_metashrew_minimal_views(&adapter, &blocks);
    
    // Test blocktracker view at different heights
    for height in 0..blocks.len() {
        let blocktracker = adapter.call_view("blocktracker", &[], Some(height as u32)).await?;
        
        // Should have (height + 1) bytes
        assert_eq!(blocktracker.len(), height + 1);
        
        // Each byte should be the hash of the corresponding block
        for (i, &hash_byte) in blocktracker.iter().enumerate() {
            let expected_hash = TestUtils::simple_hash(&blocks[i]);
            assert_eq!(hash_byte, expected_hash);
        }
        
        println!("✓ Blocktracker at height {}: {} bytes", height, blocktracker.len());
    }
    
    // Test getblock view at different heights
    for height in 0..blocks.len() {
        let height_input = (height as u32).to_le_bytes().to_vec();
        let block_data = adapter.call_view("getblock", &height_input, Some(height as u32)).await?;
        
        assert_eq!(block_data, blocks[height]);
        println!("✓ Getblock at height {}: {} bytes", height, block_data.len());
    }
    
    // Test prefix property of blocktracker
    for height in 1..blocks.len() {
        let prev_blocktracker = adapter.call_view("blocktracker", &[], Some((height - 1) as u32)).await?;
        let curr_blocktracker = adapter.call_view("blocktracker", &[], Some(height as u32)).await?;
        
        // Current should be previous + one more byte
        assert_eq!(curr_blocktracker.len(), prev_blocktracker.len() + 1);
        assert_eq!(&curr_blocktracker[..prev_blocktracker.len()], &prev_blocktracker[..]);
        
        println!("✓ Prefix property verified for height {}", height);
    }
    
    println!("✅ Metashrew view function integration test passed!");
    Ok(())
}

/// Test error handling and edge cases
#[tokio::test]
async fn test_error_handling_and_edge_cases() -> Result<()> {
    let adapter = MemoryMetashrewAdapter::new();
    
    // Test missing block hash
    let result = adapter.get_block_hash(999).await;
    assert!(result.is_err());
    println!("✓ Missing block hash error handled correctly");
    
    // Test missing view result
    let result = adapter.call_view("nonexistent_view", &[], None).await;
    assert!(result.is_err());
    println!("✓ Missing view result error handled correctly");
    
    // Test invalid view parameters
    let result = adapter.call_view("blocktracker", &[1, 2, 3], Some(0)).await;
    assert!(result.is_err());
    println!("✓ Invalid view parameters error handled correctly");
    
    // Test blockchain simulator edge cases
    let mut sim_adapter = adapter.clone();
    
    // Test reorg with invalid fork height
    sim_adapter.set_height(5);
    let result = sim_adapter.simulate_reorg(10, vec![]);
    assert!(result.is_err());
    println!("✓ Invalid reorg fork height error handled correctly");
    
    // Test empty reorg
    let result = sim_adapter.simulate_reorg(3, vec![]);
    assert!(result.is_ok());
    let (height, _) = result.unwrap();
    assert_eq!(height, 3); // Should stay at fork height
    println!("✓ Empty reorg handled correctly");
    
    println!("✅ Error handling and edge cases test passed!");
    Ok(())
}

/// Test adapter isolation and cloning
#[tokio::test]
async fn test_adapter_isolation_and_cloning() -> Result<()> {
    let adapter1 = MemoryMetashrewAdapter::with_identifier("adapter1");
    
    // Set up some data
    adapter1.set_height(42);
    adapter1.set_block_hash(42, vec![0xaa, 0xbb, 0xcc]);
    adapter1.set_view_result("test_view", &[1, 2], Some(42), vec![3, 4, 5]);
    
    // Test regular clone (shares state)
    let adapter2 = adapter1.clone();
    assert_eq!(adapter1.get_height().await?, adapter2.get_height().await?);
    
    // Modify adapter1, adapter2 should see the change
    adapter1.set_height(100);
    assert_eq!(adapter1.get_height().await?, 100);
    assert_eq!(adapter2.get_height().await?, 100); // Shared state
    
    // Test deep copy (isolated state)
    let adapter3 = adapter1.deep_copy();
    assert_eq!(adapter3.get_height().await?, 100); // Initially same
    assert_eq!(adapter3.get_identifier(), "adapter1-copy"); // Different identifier
    
    // Modify adapter1, adapter3 should not see the change
    adapter1.set_height(200);
    assert_eq!(adapter1.get_height().await?, 200);
    assert_eq!(adapter3.get_height().await?, 100); // Isolated state
    
    // Test that all data was copied
    assert_eq!(adapter3.get_block_hash(42).await?, vec![0xaa, 0xbb, 0xcc]);
    assert_eq!(adapter3.call_view("test_view", &[1, 2], Some(42)).await?, vec![3, 4, 5]);
    
    // Test clear functionality
    adapter1.clear();
    assert_eq!(adapter1.get_height().await?, 0);
    assert_eq!(adapter1.block_count(), 0);
    assert_eq!(adapter1.view_result_count(), 0);
    
    // adapter3 should still have its data
    assert_eq!(adapter3.get_height().await?, 100);
    assert_eq!(adapter3.block_count(), 1);
    assert_eq!(adapter3.view_result_count(), 1);
    
    println!("✅ Adapter isolation and cloning test passed!");
    Ok(())
}

/// Test comprehensive integration scenario
#[tokio::test]
async fn test_comprehensive_integration_scenario() -> Result<()> {
    // Create a complex scenario with multiple chains and reorgs
    let adapter = MemoryMetashrewAdapter::with_identifier("comprehensive-test");
    
    // Phase 1: Build initial chain
    let initial_blocks = TestUtils::create_test_blocks(8, "initial");
    TestUtils::setup_metashrew_minimal_views(&adapter, &initial_blocks);
    
    for (height, block_data) in initial_blocks.iter().enumerate() {
        let hash = TestUtils::simple_hash(block_data);
        adapter.set_block_hash(height as u32, vec![hash]);
    }
    adapter.set_height(7);
    
    // Verify initial state
    for height in 0..8 {
        let blocktracker = adapter.call_view("blocktracker", &[], Some(height)).await?;
        assert_eq!(blocktracker.len(), (height + 1) as usize);
        
        let height_input = (height as u32).to_le_bytes().to_vec();
        let block_data = adapter.call_view("getblock", &height_input, Some(height)).await?;
        assert_eq!(block_data, initial_blocks[height as usize]);
    }
    
    // Phase 2: Simulate blockchain progression
    let mut sim_adapter = adapter.clone();
    
    // Add more blocks
    for i in 0..3 {
        let block_data = format!("extended_block_{}", i);
        let (height, _) = sim_adapter.advance_block(Some(block_data.as_bytes()))?;
        assert_eq!(height, 8 + i);
    }
    
    assert_eq!(futures::executor::block_on(sim_adapter.get_height())?, 10);
    
    // Phase 3: Simulate reorg
    let reorg_blocks = vec![
        b"reorg_a".to_vec(),
        b"reorg_b".to_vec(),
        b"reorg_c".to_vec(),
        b"reorg_d".to_vec(),
    ];
    
    let (final_height, _) = sim_adapter.simulate_reorg(5, reorg_blocks)?;
    assert_eq!(final_height, 9); // 5 + 4 reorg blocks
    
    // Phase 4: Verify final state
    assert_eq!(futures::executor::block_on(sim_adapter.get_height())?, 9);
    assert_eq!(sim_adapter.block_count(), 10);
    
    // Blocks 0-5 should be unchanged
    for height in 0..=5 {
        let original_hash = TestUtils::simple_hash(&initial_blocks[height as usize]);
        let current_hash = futures::executor::block_on(sim_adapter.get_block_hash(height))?;
        assert_eq!(current_hash, vec![original_hash]);
    }
    
    // Blocks 6-9 should be different (reorged)
    for height in 6..=9 {
        let current_hash = futures::executor::block_on(sim_adapter.get_block_hash(height))?;
        assert!(!current_hash.is_empty());
        
        if height < initial_blocks.len() as u32 {
            let original_hash = TestUtils::simple_hash(&initial_blocks[height as usize]);
            assert_ne!(current_hash, vec![original_hash]);
        }
    }
    
    // Test health and identifier
    assert!(sim_adapter.is_healthy().await);
    assert_eq!(sim_adapter.get_identifier(), "comprehensive-test");
    
    println!("✅ Comprehensive integration scenario test passed!");
    println!("   - Initial chain: 8 blocks");
    println!("   - Extended chain: +3 blocks");
    println!("   - Reorg from height 5: +4 blocks");
    println!("   - Final height: 9");
    println!("   - All state transitions verified");
    
    Ok(())
}