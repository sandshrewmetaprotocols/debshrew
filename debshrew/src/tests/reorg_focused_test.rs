//! Focused reorg testing that validates CDC behavior during reorgs
//!
//! This test suite validates:
//! - CDC message generation during normal block processing
//! - CDC message rollback and inverse generation during reorgs
//! - State consistency across reorg scenarios
//! - Historical CDC query correctness after reorgs

use super::block_builder::{create_test_block, ChainBuilder, create_reorg_scenario};
use super::{TestConfig, TestUtils};
use crate::adapters::MemoryMetashrewAdapter;
use crate::error::Result;
use crate::traits::{MetashrewClientLike, BlockchainSimulatorLike, BlockProviderLike, ViewProviderLike};
use crate::{CdcMessage, CdcOperation};

/// Simple CDC processor that tracks block processing and generates messages
pub struct SimpleCdcProcessor {
    adapter: MemoryMetashrewAdapter,
    processed_height: Option<u32>,
    cdc_history: Vec<Vec<CdcMessage>>, // CDC messages by height
}

impl SimpleCdcProcessor {
    pub fn new() -> Self {
        Self {
            adapter: MemoryMetashrewAdapter::with_identifier("cdc-processor"),
            processed_height: None,
            cdc_history: Vec::new(),
        }
    }

    /// Process a block and generate CDC messages
    pub fn process_block(&mut self, height: u32, block_data: &[u8]) -> Result<Vec<CdcMessage>> {
        // Set up the block in our adapter
        let hash = TestUtils::simple_hash(block_data);
        self.adapter.set_block_hash(height, vec![hash]);
        self.adapter.set_height(height);
        
        // Create blocktracker data up to this height
        let mut blocktracker_data = Vec::new();
        for h in 0..=height {
            if let Ok(block_hash) = futures::executor::block_on(self.adapter.get_block_hash(h)) {
                if !block_hash.is_empty() {
                    blocktracker_data.push(block_hash[0]);
                }
            }
        }
        
        // Set up the blocktracker view
        self.adapter.set_view_result("blocktracker", &[], Some(height), blocktracker_data.clone());
        
        // Set up the getblock view
        let height_input = height.to_le_bytes().to_vec();
        self.adapter.set_view_result("getblock", &height_input, Some(height), block_data.to_vec());
        
        // Generate CDC message for this block
        let cdc_message = CdcMessage {
            header: crate::CdcHeader {
                source: "debshrew-minimal".to_string(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
                block_height: height,
                block_hash: format!("{:02x}", hash),
                transaction_id: None,
            },
            payload: crate::CdcPayload {
                operation: CdcOperation::Create,
                table: "blocks".to_string(),
                key: height.to_string(),
                before: None,
                after: Some(serde_json::json!({
                    "height": height,
                    "hash_byte": hash,
                    "tracker_length": blocktracker_data.len()
                })),
            },
        };
        
        let cdc_messages = vec![cdc_message];
        
        // Store in history
        while self.cdc_history.len() <= height as usize {
            self.cdc_history.push(Vec::new());
        }
        self.cdc_history[height as usize] = cdc_messages.clone();
        
        self.processed_height = Some(height);
        
        Ok(cdc_messages)
    }
    
    /// Process a rollback and generate inverse CDC messages
    pub fn process_rollback(&mut self, target_height: u32) -> Result<Vec<CdcMessage>> {
        let mut inverse_messages = Vec::new();
        
        if let Some(last_height) = self.processed_height {
            // Generate inverse messages for blocks that need to be rolled back
            for rollback_height in (target_height + 1)..=last_height {
                if let Some(original_messages) = self.cdc_history.get(rollback_height as usize) {
                    for original_message in original_messages {
                        let inverse_message = CdcMessage {
                            header: crate::CdcHeader {
                                source: original_message.header.source.clone(),
                                timestamp: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_millis() as u64,
                                block_height: target_height, // Target height for rollback
                                block_hash: original_message.header.block_hash.clone(),
                                transaction_id: None,
                            },
                            payload: crate::CdcPayload {
                                operation: CdcOperation::Delete,
                                table: original_message.payload.table.clone(),
                                key: original_message.payload.key.clone(),
                                before: original_message.payload.after.clone(),
                                after: None,
                            },
                        };
                        
                        inverse_messages.push(inverse_message);
                    }
                }
                
                // Clear the CDC history for this height
                if rollback_height < self.cdc_history.len() as u32 {
                    self.cdc_history[rollback_height as usize].clear();
                }
            }
        }
        
        self.processed_height = Some(target_height);
        
        Ok(inverse_messages)
    }
    
    /// Get CDC messages for a specific height
    pub fn get_cdc_messages(&self, height: u32) -> Vec<CdcMessage> {
        self.cdc_history.get(height as usize).cloned().unwrap_or_default()
    }
    
    /// Get the adapter for testing
    pub fn get_adapter(&self) -> &MemoryMetashrewAdapter {
        &self.adapter
    }
    
    pub fn get_processed_height(&self) -> Option<u32> {
        self.processed_height
    }
}

/// Test reorg scenario: build chain, then rebuild part of it with different blocks
#[tokio::test]
async fn test_reorg_scenario_cdc_consistency() -> Result<()> {
    let mut processor = SimpleCdcProcessor::new();

    // Phase 1: Build initial chain (4 blocks)
    let initial_blocks = TestUtils::create_test_blocks(4, "initial");

    for (height, block_data) in initial_blocks.iter().enumerate() {
        let cdc_messages = processor.process_block(height as u32, block_data)?;
        assert_eq!(cdc_messages.len(), 1);
        assert_eq!(cdc_messages[0].header.block_height, height as u32);
        assert_eq!(cdc_messages[0].payload.operation, CdcOperation::Create);
    }
    
    assert_eq!(processor.get_processed_height(), Some(3));

    // Store initial CDC states
    let mut initial_cdc_states = Vec::new();
    for height in 0..=3 {
        let cdc_messages = processor.get_cdc_messages(height);
        initial_cdc_states.push(cdc_messages);
    }

    // Phase 2: Simulate reorg by rolling back and reprocessing
    // Roll back to height 1 (keeping blocks 0 and 1)
    let rollback_messages = processor.process_rollback(1)?;
    
    // Should have 2 inverse messages (for heights 2 and 3)
    assert_eq!(rollback_messages.len(), 2);
    for (i, message) in rollback_messages.iter().enumerate() {
        assert_eq!(message.payload.operation, CdcOperation::Delete);
        assert_eq!(message.payload.key, (2 + i).to_string()); // Heights 2 and 3
        assert_eq!(message.header.block_height, 1); // Target rollback height
    }

    // Phase 3: Process new blocks for the reorg
    let reorg_blocks = TestUtils::create_test_blocks(3, "reorg"); // 3 new blocks
    
    // Process new blocks starting from height 2
    for (i, block_data) in reorg_blocks.iter().enumerate() {
        let height = 2 + i as u32;
        let cdc_messages = processor.process_block(height, block_data)?;
        assert_eq!(cdc_messages.len(), 1);
        assert_eq!(cdc_messages[0].header.block_height, height);
        assert_eq!(cdc_messages[0].payload.operation, CdcOperation::Create);
    }

    // Phase 4: Verify reorg results
    println!("\n=== Verifying reorg results ===");

    // Heights 0-1 should have the same CDC messages (unaffected by reorg)
    for height in 0..2 {
        let original_cdc = &initial_cdc_states[height];
        let reorg_cdc = processor.get_cdc_messages(height as u32);
        assert_eq!(original_cdc.len(), reorg_cdc.len());
        if !original_cdc.is_empty() && !reorg_cdc.is_empty() {
            assert_eq!(original_cdc[0].payload.key, reorg_cdc[0].payload.key);
            assert_eq!(original_cdc[0].payload.operation, reorg_cdc[0].payload.operation);
        }
        println!("✓ Height {} CDC unchanged: {} messages", height, reorg_cdc.len());
    }

    // Heights 2-4 should have different CDC messages (affected by reorg)
    for height in 2..=4 {
        let reorg_cdc = processor.get_cdc_messages(height);
        assert_eq!(reorg_cdc.len(), 1);
        assert_eq!(reorg_cdc[0].payload.operation, CdcOperation::Create);
        assert_eq!(reorg_cdc[0].payload.key, height.to_string());
        
        // Compare with original if it exists
        if height < initial_cdc_states.len() as u32 {
            let original_cdc = &initial_cdc_states[height as usize];
            if !original_cdc.is_empty() {
                // The block hash should be different due to different block content
                assert_ne!(original_cdc[0].header.block_hash, reorg_cdc[0].header.block_hash);
            }
        }
        
        println!("✓ Height {} CDC changed: {} messages", height, reorg_cdc.len());
    }

    println!("✅ Reorg scenario CDC consistency test passed!");
    Ok(())
}

/// Test deep reorg scenario
#[tokio::test]
async fn test_deep_reorg_scenario() -> Result<()> {
    let mut processor = SimpleCdcProcessor::new();

    // Build initial chain (5 blocks)
    let initial_blocks = TestUtils::create_test_blocks(5, "initial");

    for (height, block_data) in initial_blocks.iter().enumerate() {
        processor.process_block(height as u32, block_data)?;
    }

    // Create deep reorg: replace everything from height 1 onwards
    let rollback_messages = processor.process_rollback(0)?; // Roll back to genesis only
    
    // Should have 4 inverse messages (for heights 1, 2, 3, 4)
    assert_eq!(rollback_messages.len(), 4);

    // Process new reorg chain
    let reorg_blocks = TestUtils::create_test_blocks(6, "deep_reorg"); // 6 new blocks
    
    for (i, block_data) in reorg_blocks.iter().enumerate() {
        let height = 1 + i as u32; // Start from height 1
        processor.process_block(height, block_data)?;
    }

    // Verify results
    // Height 0 (genesis) should be unchanged
    let genesis_cdc = processor.get_cdc_messages(0);
    assert_eq!(genesis_cdc.len(), 1);
    assert_eq!(genesis_cdc[0].payload.key, "0");

    // Heights 1-6 should all have new CDC messages
    for height in 1..=6 {
        let cdc_messages = processor.get_cdc_messages(height);
        assert_eq!(cdc_messages.len(), 1);
        assert_eq!(cdc_messages[0].payload.operation, CdcOperation::Create);
        assert_eq!(cdc_messages[0].payload.key, height.to_string());
    }

    println!("✅ Deep reorg scenario test passed!");
    Ok(())
}

/// Test CDC message structure and content
#[test]
fn test_cdc_message_structure() -> Result<()> {
    let mut processor = SimpleCdcProcessor::new();
    
    // Process a single block
    let block_data = create_test_block(0, b"test_block");
    let cdc_messages = processor.process_block(0, &block_data)?;
    
    assert_eq!(cdc_messages.len(), 1);
    let message = &cdc_messages[0];
    
    // Verify header structure
    assert_eq!(message.header.source, "debshrew-minimal");
    assert_eq!(message.header.block_height, 0);
    assert!(!message.header.block_hash.is_empty());
    assert!(message.header.timestamp > 0);
    assert!(message.header.transaction_id.is_none());
    
    // Verify payload structure
    assert_eq!(message.payload.operation, CdcOperation::Create);
    assert_eq!(message.payload.table, "blocks");
    assert_eq!(message.payload.key, "0");
    assert!(message.payload.before.is_none());
    assert!(message.payload.after.is_some());
    
    // Verify payload content
    if let Some(after) = &message.payload.after {
        assert_eq!(after["height"], 0);
        assert!(after["hash_byte"].is_number());
        assert_eq!(after["tracker_length"], 1);
    }
    
    println!("✅ CDC message structure test passed!");
    Ok(())
}

/// Test rollback CDC message generation
#[test]
fn test_rollback_cdc_generation() -> Result<()> {
    let mut processor = SimpleCdcProcessor::new();
    
    // Process 3 blocks
    for height in 0..3 {
        let block_data = create_test_block(height, format!("block_{}", height).as_bytes());
        processor.process_block(height, &block_data)?;
    }
    
    // Roll back to height 0
    let rollback_messages = processor.process_rollback(0)?;
    
    // Should have 2 inverse messages (for heights 1 and 2)
    assert_eq!(rollback_messages.len(), 2);
    
    for (i, message) in rollback_messages.iter().enumerate() {
        let expected_height = 1 + i as u32;
        
        // Verify inverse message structure
        assert_eq!(message.header.source, "debshrew-minimal");
        assert_eq!(message.header.block_height, 0); // Target rollback height
        assert_eq!(message.payload.operation, CdcOperation::Delete);
        assert_eq!(message.payload.table, "blocks");
        assert_eq!(message.payload.key, expected_height.to_string());
        assert!(message.payload.before.is_some()); // Should have the original data
        assert!(message.payload.after.is_none());
    }
    
    println!("✅ Rollback CDC generation test passed!");
    Ok(())
}

/// Test comprehensive reorg scenario with all verifications
#[tokio::test]
async fn test_comprehensive_reorg_all_verifications() -> Result<()> {
    let mut processor = SimpleCdcProcessor::new();

    // Phase 1: Build initial chain (4 blocks)
    let initial_blocks = TestUtils::create_test_blocks(4, "initial");

    for (height, block_data) in initial_blocks.iter().enumerate() {
        processor.process_block(height as u32, block_data)?;
    }

    // Capture initial CDC state
    let mut initial_cdc_states = Vec::new();
    for height in 0..4 {
        initial_cdc_states.push(processor.get_cdc_messages(height));
    }

    // Phase 2: Create reorg - replace blocks 1, 2, 3
    let rollback_messages = processor.process_rollback(0)?;
    assert_eq!(rollback_messages.len(), 3); // Should roll back 3 blocks

    // Process new reorg blocks
    let reorg_blocks = TestUtils::create_test_blocks(4, "reorg");
    for (i, block_data) in reorg_blocks.iter().enumerate() {
        let height = 1 + i as u32; // Start from height 1
        processor.process_block(height, block_data)?;
    }

    // Phase 3: Comprehensive verification

    // 1. Verify rollback messages were correct
    for (i, rollback_msg) in rollback_messages.iter().enumerate() {
        assert_eq!(rollback_msg.payload.operation, CdcOperation::Delete);
        assert_eq!(rollback_msg.payload.key, (1 + i).to_string());
        assert_eq!(rollback_msg.header.block_height, 0); // Target height
    }

    // 2. Verify genesis block unchanged
    let genesis_cdc = processor.get_cdc_messages(0);
    let initial_genesis_cdc = &initial_cdc_states[0];
    assert_eq!(genesis_cdc.len(), initial_genesis_cdc.len());
    if !genesis_cdc.is_empty() && !initial_genesis_cdc.is_empty() {
        assert_eq!(genesis_cdc[0].payload.key, initial_genesis_cdc[0].payload.key);
    }

    // 3. Verify reorg blocks have new CDC messages
    for height in 1..=4 {
        let reorg_cdc = processor.get_cdc_messages(height);
        assert_eq!(reorg_cdc.len(), 1);
        assert_eq!(reorg_cdc[0].payload.operation, CdcOperation::Create);
        assert_eq!(reorg_cdc[0].payload.key, height.to_string());
        
        // Should be different from original (if it existed)
        if height < initial_cdc_states.len() as u32 {
            let original_cdc = &initial_cdc_states[height as usize];
            if !original_cdc.is_empty() {
                assert_ne!(original_cdc[0].header.block_hash, reorg_cdc[0].header.block_hash);
            }
        }
    }

    // 4. Verify adapter state consistency
    let adapter = processor.get_adapter();
    assert_eq!(adapter.get_height().await?, 4);
    
    for height in 0..=4 {
        let hash = adapter.get_block_hash(height).await?;
        assert!(!hash.is_empty());
        
        let blocktracker = adapter.call_view("blocktracker", &[], Some(height)).await?;
        assert_eq!(blocktracker.len(), (height + 1) as usize);
    }

    println!("✅ Comprehensive reorg verification with all checks passed!");
    println!("   - Rollback CDC messages verified");
    println!("   - Genesis block unchanged");
    println!   ("   - Reorg blocks have new CDC messages");
    println!("   - Adapter state consistency verified");

    Ok(())
}