//! Block builder utilities for creating test scenarios
//!
//! This module provides utilities for building test block sequences,
//! simulating chain reorganizations, and creating realistic test data.

use std::collections::VecDeque;

/// A builder for creating test block sequences
pub struct BlockBuilder {
    height: u32,
    content: Vec<u8>,
    timestamp: u64,
}

impl BlockBuilder {
    /// Create a new block builder
    pub fn new() -> Self {
        Self {
            height: 0,
            content: Vec::new(),
            timestamp: 1231006505, // Bitcoin genesis timestamp
        }
    }

    /// Set the block height
    pub fn height(mut self, height: u32) -> Self {
        self.height = height;
        self.timestamp = 1231006505 + (height as u64 * 600); // 10 minutes per block
        self
    }

    /// Set custom content for the block
    pub fn content(mut self, content: &[u8]) -> Self {
        self.content = content.to_vec();
        self
    }

    /// Set the timestamp
    pub fn timestamp(mut self, timestamp: u64) -> Self {
        self.timestamp = timestamp;
        self
    }

    /// Build the block data
    pub fn build(self) -> Vec<u8> {
        let mut block_data = Vec::new();
        
        // Add height as first 4 bytes
        block_data.extend_from_slice(&self.height.to_le_bytes());
        
        // Add timestamp as next 8 bytes
        block_data.extend_from_slice(&self.timestamp.to_le_bytes());
        
        // Add content
        if self.content.is_empty() {
            // Default content based on height
            let default_content = format!("block_{}", self.height);
            block_data.extend_from_slice(default_content.as_bytes());
        } else {
            block_data.extend_from_slice(&self.content);
        }
        
        // Pad to minimum size
        while block_data.len() < 64 {
            block_data.push(0);
        }
        
        block_data
    }
}

/// A chain builder for creating sequences of test blocks
pub struct ChainBuilder {
    blocks: VecDeque<Vec<u8>>,
    current_height: u32,
    base_timestamp: u64,
}

impl ChainBuilder {
    /// Create a new chain builder starting with genesis
    pub fn new() -> Self {
        let genesis = BlockBuilder::new()
            .height(0)
            .content(b"genesis_block")
            .build();

        let mut blocks = VecDeque::new();
        blocks.push_back(genesis);

        Self {
            blocks,
            current_height: 0,
            base_timestamp: 1231006505,
        }
    }

    /// Add a block to the chain
    pub fn add_block(mut self) -> Self {
        let next_height = self.current_height + 1;
        let block = BlockBuilder::new()
            .height(next_height)
            .timestamp(self.base_timestamp + (next_height as u64 * 600))
            .build();

        self.current_height = next_height;
        self.blocks.push_back(block);

        self
    }

    /// Add multiple blocks to the chain
    pub fn add_blocks(mut self, count: u32) -> Self {
        for _ in 0..count {
            self = self.add_block();
        }
        self
    }

    /// Add a custom block to the chain
    pub fn add_custom_block<F>(mut self, builder_fn: F) -> Self
    where
        F: FnOnce(BlockBuilder) -> BlockBuilder,
    {
        let next_height = self.current_height + 1;
        let block = builder_fn(
            BlockBuilder::new()
                .height(next_height)
                .timestamp(self.base_timestamp + (next_height as u64 * 600))
        )
        .build();

        self.current_height = next_height;
        self.blocks.push_back(block);

        self
    }

    /// Get all blocks in the chain
    pub fn blocks(self) -> Vec<Vec<u8>> {
        self.blocks.into()
    }

    /// Get the current height
    pub fn height(&self) -> u32 {
        self.current_height
    }

    /// Get a specific block by height
    pub fn get_block(&self, height: u32) -> Option<&Vec<u8>> {
        self.blocks.get(height as usize)
    }
}

/// Simple function to create a test block with given parameters
pub fn create_test_block(height: u32, content: &[u8]) -> Vec<u8> {
    BlockBuilder::new()
        .height(height)
        .content(content)
        .build()
}

/// Create a test block sequence for reorg scenarios
pub fn create_reorg_scenario(
    common_height: u32,
    original_blocks: u32,
    reorg_blocks: u32,
) -> (Vec<Vec<u8>>, Vec<Vec<u8>>) {
    // Create common chain up to fork point
    let mut common_chain = ChainBuilder::new();
    for _ in 0..common_height {
        common_chain = common_chain.add_block();
    }
    let common_blocks = common_chain.blocks();

    // Create original chain
    let mut original_chain = ChainBuilder::new();
    for _ in 0..common_height {
        original_chain = original_chain.add_block();
    }
    for i in 0..original_blocks {
        original_chain = original_chain.add_custom_block(|builder| {
            builder.content(format!("original_block_{}", i).as_bytes())
        });
    }
    let original_full = original_chain.blocks();

    // Create reorg chain
    let mut reorg_chain = ChainBuilder::new();
    for _ in 0..common_height {
        reorg_chain = reorg_chain.add_block();
    }
    for i in 0..reorg_blocks {
        reorg_chain = reorg_chain.add_custom_block(|builder| {
            builder.content(format!("reorg_block_{}", i).as_bytes())
        });
    }
    let reorg_full = reorg_chain.blocks();

    (original_full, reorg_full)
}

/// Extract height from block data
pub fn extract_height(block_data: &[u8]) -> Option<u32> {
    if block_data.len() >= 4 {
        let height_bytes: [u8; 4] = block_data[0..4].try_into().ok()?;
        Some(u32::from_le_bytes(height_bytes))
    } else {
        None
    }
}

/// Extract timestamp from block data
pub fn extract_timestamp(block_data: &[u8]) -> Option<u64> {
    if block_data.len() >= 12 {
        let timestamp_bytes: [u8; 8] = block_data[4..12].try_into().ok()?;
        Some(u64::from_le_bytes(timestamp_bytes))
    } else {
        None
    }
}

/// Extract content from block data
pub fn extract_content(block_data: &[u8]) -> Option<&[u8]> {
    if block_data.len() > 12 {
        // Find the end of actual content (before padding zeros)
        let content_start = 12;
        let content_slice = &block_data[content_start..];
        
        // Find the first sequence of zeros that indicates padding
        if let Some(padding_start) = content_slice.windows(4).position(|w| w == [0, 0, 0, 0]) {
            Some(&content_slice[..padding_start])
        } else {
            Some(content_slice)
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_builder() {
        let block = BlockBuilder::new()
            .height(5)
            .content(b"test_content")
            .build();

        assert!(block.len() >= 64);
        assert_eq!(extract_height(&block), Some(5));
        assert_eq!(extract_content(&block), Some(b"test_content".as_slice()));
    }

    #[test]
    fn test_chain_builder() {
        let chain = ChainBuilder::new().add_blocks(3).blocks();

        assert_eq!(chain.len(), 4); // Genesis + 3 blocks

        // Verify chain integrity
        for (i, block) in chain.iter().enumerate() {
            assert_eq!(extract_height(block), Some(i as u32));
        }
    }

    #[test]
    fn test_chain_builder_custom_block() {
        let chain = ChainBuilder::new()
            .add_custom_block(|builder| builder.content(b"custom_content"))
            .blocks();

        assert_eq!(chain.len(), 2); // Genesis + custom block
        assert_eq!(extract_content(&chain[1]), Some(b"custom_content".as_slice()));
    }

    #[test]
    fn test_reorg_scenario() {
        let (original, reorg) = create_reorg_scenario(2, 2, 3);

        // Both chains should have the same first 3 blocks (genesis + 2 common)
        assert_eq!(original.len(), 5); // Genesis + 2 common + 2 original
        assert_eq!(reorg.len(), 6);    // Genesis + 2 common + 3 reorg

        // Common blocks should be identical
        for i in 0..3 {
            assert_eq!(original[i], reorg[i]);
        }

        // Blocks after fork should be different
        assert_ne!(original[3], reorg[3]);
    }

    #[test]
    fn test_block_data_extraction() {
        let block = BlockBuilder::new()
            .height(42)
            .timestamp(1234567890)
            .content(b"hello_world")
            .build();

        assert_eq!(extract_height(&block), Some(42));
        assert_eq!(extract_timestamp(&block), Some(1234567890));
        assert_eq!(extract_content(&block), Some(b"hello_world".as_slice()));
    }

    #[test]
    fn test_create_test_block() {
        let block = create_test_block(10, b"test");
        assert_eq!(extract_height(&block), Some(10));
        assert_eq!(extract_content(&block), Some(b"test".as_slice()));
    }
}