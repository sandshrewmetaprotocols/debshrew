//! Block cache and related types
//!
//! This module defines the block cache and related types for handling blocks
//! and chain reorganizations.

use crate::error::{Error, Result};
use debshrew_runtime::TransformResult;
use debshrew_support::{BlockMetadata, CdcMessage, TransformState};
use std::collections::VecDeque;

/// Block cache
///
/// The block cache maintains a cache of recent blocks, including state snapshots
/// and CDC messages, to handle chain reorganizations.
#[derive(Debug)]
pub struct BlockCache {
    /// The maximum number of blocks to cache
    max_size: u32,
    
    /// The cached blocks
    blocks: VecDeque<CachedBlock>,
}

/// Cached block
///
/// A cached block includes the block metadata, state snapshot, and CDC messages.
#[derive(Debug, Clone)]
pub struct CachedBlock {
    /// The block metadata
    pub metadata: BlockMetadata,
    
    /// The state snapshot after processing this block
    pub state_snapshot: TransformState,
    
    /// The CDC messages generated for this block
    pub cdc_messages: Vec<CdcMessage>,
}

impl BlockCache {
    /// Create a new block cache
    ///
    /// # Arguments
    ///
    /// * `max_size` - The maximum number of blocks to cache
    ///
    /// # Returns
    ///
    /// A new block cache
    ///
    /// # Errors
    ///
    /// Returns an error if the max_size is 0
    pub fn new(max_size: u32) -> Result<Self> {
        if max_size == 0 {
            return Err(Error::BlockSynchronization("Cache size must be greater than 0".to_string()));
        }
        
        Ok(Self {
            max_size,
            blocks: VecDeque::with_capacity(max_size as usize),
        })
    }
    
    /// Add a block to the cache
    ///
    /// # Arguments
    ///
    /// * `metadata` - The block metadata
    /// * `transform_result` - The result of processing the block
    ///
    /// # Returns
    ///
    /// Ok(()) if the block was added successfully
    ///
    /// # Errors
    ///
    /// Returns an error if the block cannot be added
    pub fn add_block(
        &mut self,
        metadata: BlockMetadata,
        transform_result: TransformResult,
    ) -> Result<()> {
        // Create a cached block
        let cached_block = CachedBlock {
            metadata,
            state_snapshot: transform_result.state_snapshot,
            cdc_messages: transform_result.cdc_messages,
        };
        
        // Add the block to the cache
        self.blocks.push_back(cached_block);
        
        // Remove the oldest block if the cache is full
        if self.blocks.len() > self.max_size as usize {
            self.blocks.pop_front();
        }
        
        Ok(())
    }
    
    /// Get the latest block in the cache
    ///
    /// # Returns
    ///
    /// The latest block in the cache, or None if the cache is empty
    pub fn get_latest_block(&self) -> Option<&CachedBlock> {
        self.blocks.back()
    }
    
    /// Get the block at a specific height
    ///
    /// # Arguments
    ///
    /// * `height` - The block height
    ///
    /// # Returns
    ///
    /// The block at the specified height, or None if the block is not in the cache
    pub fn get_block_at_height(&self, height: u32) -> Option<&CachedBlock> {
        self.blocks.iter().find(|block| block.metadata.height == height)
    }
    
    /// Get the block with a specific hash
    ///
    /// # Arguments
    ///
    /// * `hash` - The block hash
    ///
    /// # Returns
    ///
    /// The block with the specified hash, or None if the block is not in the cache
    pub fn get_block_with_hash(&self, hash: &str) -> Option<&CachedBlock> {
        self.blocks.iter().find(|block| block.metadata.hash == hash)
    }
    
    /// Get the block hash at a specific height
    ///
    /// # Arguments
    ///
    /// * `height` - The block height
    ///
    /// # Returns
    ///
    /// The block hash at the specified height, or None if the block is not in the cache
    pub fn get_block_hash(&self, height: u32) -> Option<String> {
        self.get_block_at_height(height).map(|block| block.metadata.hash.clone())
    }
    
    /// Get the state snapshot at a specific height
    ///
    /// # Arguments
    ///
    /// * `height` - The block height
    ///
    /// # Returns
    ///
    /// The state snapshot at the specified height, or None if the block is not in the cache
    pub fn get_state_snapshot(&self, height: u32) -> Option<TransformState> {
        self.get_block_at_height(height).map(|block| block.state_snapshot.clone())
    }
    /// Find the common ancestor between the current chain and a new chain
    ///
    /// # Arguments
    ///
    /// * `new_hashes` - The block hashes of the new chain, ordered by height
    ///
    /// # Returns
    ///
    /// The height of the highest common ancestor, or None if there is no common ancestor
    pub fn find_common_ancestor(&self, new_hashes: &[(u32, String)]) -> Option<u32> {
        // Sort the new hashes by height in descending order
        let mut sorted_hashes = new_hashes.to_vec();
        sorted_hashes.sort_by_key(|(height, _)| std::cmp::Reverse(*height));
        
        // Find the highest common ancestor
        for (height, hash) in sorted_hashes {
            if let Some(cached_hash) = self.get_block_hash(height) {
                if cached_hash == hash {
                    return Some(height);
                }
            }
        }
        
        None
    }
    
    /// Roll back to a specific height
    ///
    /// # Arguments
    ///
    /// * `height` - The height to roll back to
    ///
    /// # Returns
    ///
    /// The state snapshot at the specified height
    ///
    /// # Errors
    ///
    /// Returns an error if the height is not in the cache
    pub fn rollback(&mut self, height: u32) -> Result<TransformState> {
        // Find the index of the block at the specified height
        let index = self.blocks.iter().position(|block| block.metadata.height == height)
            .ok_or_else(|| Error::ReorgHandling(format!("Block at height {} not found in cache", height)))?;
        
        // Remove all blocks after the specified height
        self.blocks.truncate(index + 1);
        
        // Return the state snapshot at the specified height
        Ok(self.blocks[index].state_snapshot.clone())
    }
    
    /// Get all CDC messages in the cache
    ///
    /// # Returns
    ///
    /// All CDC messages in the cache, ordered by block height
    pub fn get_all_cdc_messages(&self) -> Vec<CdcMessage> {
        let mut messages = Vec::new();
        
        for block in &self.blocks {
            messages.extend(block.cdc_messages.clone());
        }
        
        messages
    }
    
    /// Get the CDC messages for a specific block
    ///
    /// # Arguments
    ///
    /// * `height` - The block height
    ///
    /// # Returns
    ///
    /// The CDC messages for the specified block, or None if the block is not in the cache
    pub fn get_cdc_messages(&self, height: u32) -> Option<Vec<CdcMessage>> {
        self.get_block_at_height(height).map(|block| block.cdc_messages.clone())
    }
    
    /// Get the CDC messages for a range of blocks
    ///
    /// # Arguments
    ///
    /// * `start_height` - The start height (inclusive)
    /// * `end_height` - The end height (inclusive)
    ///
    /// # Returns
    ///
    /// The CDC messages for the specified range of blocks
    pub fn get_cdc_messages_range(&self, start_height: u32, end_height: u32) -> Vec<CdcMessage> {
        let mut messages = Vec::new();
        
        for block in &self.blocks {
            let height = block.metadata.height;
            if height >= start_height && height <= end_height {
                messages.extend(block.cdc_messages.clone());
            }
        }
        
        messages
    }
    
    /// Clear the cache
    pub fn clear(&mut self) {
        self.blocks.clear();
    }
    
    /// Get the number of blocks in the cache
    ///
    /// # Returns
    ///
    /// The number of blocks in the cache
    pub fn len(&self) -> usize {
        self.blocks.len()
    }
    
    /// Check if the cache is empty
    ///
    /// # Returns
    ///
    /// True if the cache is empty, false otherwise
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }
    
    /// Get the maximum size of the cache
    ///
    /// # Returns
    ///
    /// The maximum number of blocks that can be stored in the cache
    pub fn max_size(&self) -> u32 {
        self.max_size
    }
    
    /// Get the lowest block height in the cache
    ///
    /// # Returns
    ///
    /// The lowest block height in the cache, or None if the cache is empty
    pub fn lowest_height(&self) -> Option<u32> {
        self.blocks.front().map(|block| block.metadata.height)
    }
    
    /// Get the highest block height in the cache
    ///
    /// # Returns
    ///
    /// The highest block height in the cache, or None if the cache is empty
    pub fn highest_height(&self) -> Option<u32> {
        self.blocks.back().map(|block| block.metadata.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use debshrew_support::{CdcHeader, CdcOperation, CdcPayload};

    fn create_test_block(height: u32, hash: &str) -> (BlockMetadata, TransformResult) {
        let metadata = BlockMetadata {
            height,
            hash: hash.to_string(),
            timestamp: Utc::now().timestamp_millis() as u64,
        };
        
        let cdc_message = CdcMessage {
            header: CdcHeader {
                source: "test".to_string(),
                timestamp: Utc::now().timestamp_millis() as u64,
                block_height: height,
                block_hash: hash.to_string(),
                transaction_id: None,
            },
            payload: CdcPayload {
                operation: CdcOperation::Create,
                table: "test_table".to_string(),
                key: format!("key_{}", height),
                before: None,
                after: Some(serde_json::json!({
                    "height": height,
                    "hash": hash
                })),
            },
        };
        
        let transform_result = TransformResult {
            cdc_messages: vec![cdc_message],
            state_snapshot: TransformState::new(),
        };
        
        (metadata, transform_result)
    }

    #[test]
    fn test_block_cache() {
        // Create a block cache
        let mut cache = BlockCache::new(3).unwrap();
        
        // Add some blocks
        let (metadata1, result1) = create_test_block(1, "hash1");
        let (metadata2, result2) = create_test_block(2, "hash2");
        let (metadata3, result3) = create_test_block(3, "hash3");
        
        cache.add_block(metadata1.clone(), result1.clone()).unwrap();
        cache.add_block(metadata2.clone(), result2.clone()).unwrap();
        cache.add_block(metadata3.clone(), result3.clone()).unwrap();
        
        // Check the cache size
        assert_eq!(cache.len(), 3);
        assert_eq!(cache.max_size(), 3);
        assert_eq!(cache.lowest_height(), Some(1));
        assert_eq!(cache.highest_height(), Some(3));
        
        // Get blocks by height
        let block1 = cache.get_block_at_height(1).unwrap();
        let block2 = cache.get_block_at_height(2).unwrap();
        let block3 = cache.get_block_at_height(3).unwrap();
        
        assert_eq!(block1.metadata.height, 1);
        assert_eq!(block1.metadata.hash, "hash1");
        assert_eq!(block2.metadata.height, 2);
        assert_eq!(block2.metadata.hash, "hash2");
        assert_eq!(block3.metadata.height, 3);
        assert_eq!(block3.metadata.hash, "hash3");
        
        // Get blocks by hash
        let block1_by_hash = cache.get_block_with_hash("hash1").unwrap();
        let block2_by_hash = cache.get_block_with_hash("hash2").unwrap();
        let block3_by_hash = cache.get_block_with_hash("hash3").unwrap();
        
        assert_eq!(block1_by_hash.metadata.height, 1);
        assert_eq!(block2_by_hash.metadata.height, 2);
        assert_eq!(block3_by_hash.metadata.height, 3);
        
        // Get block hashes
        assert_eq!(cache.get_block_hash(1), Some("hash1".to_string()));
        assert_eq!(cache.get_block_hash(2), Some("hash2".to_string()));
        assert_eq!(cache.get_block_hash(3), Some("hash3".to_string()));
        
        // Get CDC messages
        let messages1 = cache.get_cdc_messages(1).unwrap();
        let messages2 = cache.get_cdc_messages(2).unwrap();
        let messages3 = cache.get_cdc_messages(3).unwrap();
        
        assert_eq!(messages1.len(), 1);
        assert_eq!(messages1[0].payload.key, "key_1");
        assert_eq!(messages2.len(), 1);
        assert_eq!(messages2[0].payload.key, "key_2");
        assert_eq!(messages3.len(), 1);
        assert_eq!(messages3[0].payload.key, "key_3");
        
        // Get CDC messages for a range
        let messages_range = cache.get_cdc_messages_range(1, 2);
        assert_eq!(messages_range.len(), 2);
        assert_eq!(messages_range[0].payload.key, "key_1");
        assert_eq!(messages_range[1].payload.key, "key_2");
        
        // Get all CDC messages
        let all_messages = cache.get_all_cdc_messages();
        assert_eq!(all_messages.len(), 3);
        
        // Test cache eviction
        let (metadata4, result4) = create_test_block(4, "hash4");
        cache.add_block(metadata4.clone(), result4.clone()).unwrap();
        
        assert_eq!(cache.len(), 3);
        assert_eq!(cache.lowest_height(), Some(2));
        assert_eq!(cache.highest_height(), Some(4));
        
        // Block 1 should be evicted
        assert!(cache.get_block_at_height(1).is_none());
        
        // Test rollback
        let _state = cache.rollback(2).unwrap();
        
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.lowest_height(), Some(2));
        assert_eq!(cache.highest_height(), Some(2));
        
        // Blocks 3 and 4 should be removed
        assert!(cache.get_block_at_height(3).is_none());
        assert!(cache.get_block_at_height(4).is_none());
        
        // Test clear
        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_find_common_ancestor() {
        // Create a block cache
        let mut cache = BlockCache::new(5).unwrap();
        
        // Add some blocks
        let (metadata1, result1) = create_test_block(1, "hash1");
        let (metadata2, result2) = create_test_block(2, "hash2");
        let (metadata3, result3) = create_test_block(3, "hash3");
        let (metadata4, result4) = create_test_block(4, "hash4");
        let (metadata5, result5) = create_test_block(5, "hash5");
        
        cache.add_block(metadata1.clone(), result1.clone()).unwrap();
        cache.add_block(metadata2.clone(), result2.clone()).unwrap();
        cache.add_block(metadata3.clone(), result3.clone()).unwrap();
        cache.add_block(metadata4.clone(), result4.clone()).unwrap();
        cache.add_block(metadata5.clone(), result5.clone()).unwrap();
        
        // Test common ancestor at height 3
        let new_hashes = vec![
            (1, "hash1".to_string()),
            (2, "hash2".to_string()),
            (3, "hash3".to_string()),
            (4, "newhash4".to_string()),
            (5, "newhash5".to_string()),
        ];
        
        let ancestor = cache.find_common_ancestor(&new_hashes).unwrap();
        assert_eq!(ancestor, 3);
        
        // Test common ancestor at height 1
        let new_hashes = vec![
            (1, "hash1".to_string()),
            (2, "newhash2".to_string()),
            (3, "newhash3".to_string()),
        ];
        
        let ancestor = cache.find_common_ancestor(&new_hashes).unwrap();
        assert_eq!(ancestor, 1);
        
        // Test no common ancestor
        let new_hashes = vec![
            (1, "newhash1".to_string()),
            (2, "newhash2".to_string()),
        ];
        
        let ancestor = cache.find_common_ancestor(&new_hashes);
        assert!(ancestor.is_none());
    }
}