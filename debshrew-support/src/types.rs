//! Core data types for debshrew
//!
//! This module defines the core data types used throughout the debshrew project,
//! including CDC message types, state types, and other shared data structures.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// CDC message header
///
/// Contains metadata about the CDC message, including the source, timestamp,
/// block height, block hash, and transaction ID.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CdcHeader {
    /// Source of the CDC message (e.g., "debshrew", "token_protocol")
    pub source: String,
    
    /// Timestamp when the CDC message was generated (milliseconds since UNIX epoch)
    pub timestamp: u64,
    
    /// Block height where the change occurred
    pub block_height: u32,
    
    /// Block hash where the change occurred
    pub block_hash: String,
    
    /// Transaction ID where the change occurred (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_id: Option<String>,
}

/// CDC message payload
///
/// Contains the actual data for the CDC message, including the operation type,
/// table, key, and before/after states.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CdcPayload {
    /// Operation type (create, update, delete)
    pub operation: CdcOperation,
    
    /// Table name
    pub table: String,
    
    /// Record key
    pub key: String,
    
    /// State before the operation (null for create operations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<serde_json::Value>,
    
    /// State after the operation (null for delete operations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<serde_json::Value>,
}

/// CDC message
///
/// A complete CDC message, including header and payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CdcMessage {
    /// Message header
    pub header: CdcHeader,
    
    /// Message payload
    pub payload: CdcPayload,
}

/// CDC operation type
///
/// Represents the type of operation that generated the CDC message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CdcOperation {
    /// Create operation (new record)
    Create,
    
    /// Update operation (modified record)
    Update,
    
    /// Delete operation (removed record)
    Delete,
}

/// Transform state
///
/// Represents the state of a transform module, which is a key-value store
/// that persists between block processing.
#[derive(Debug, Clone, Default)]
pub struct TransformState {
    /// Inner state storage
    inner: HashMap<Vec<u8>, Vec<u8>>,
    
    /// Whether the state has been modified
    dirty: bool,
}

impl TransformState {
    /// Create a new transform state
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
            dirty: false,
        }
    }
    
    /// Get a value from the state
    ///
    /// # Arguments
    ///
    /// * `key` - The key to get
    ///
    /// # Returns
    ///
    /// The value associated with the key, or None if the key doesn't exist
    pub fn get(&self, key: &[u8]) -> Option<&Vec<u8>> {
        self.inner.get(key)
    }
    
    /// Set a value in the state
    ///
    /// # Arguments
    ///
    /// * `key` - The key to set
    /// * `value` - The value to set
    pub fn set(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.inner.insert(key, value);
        self.dirty = true;
    }
    
    /// Delete a value from the state
    ///
    /// # Arguments
    ///
    /// * `key` - The key to delete
    ///
    /// # Returns
    ///
    /// True if the key existed and was removed, false otherwise
    pub fn delete(&mut self, key: &[u8]) -> bool {
        let result = self.inner.remove(key).is_some();
        if result {
            self.dirty = true;
        }
        result
    }
    
    /// Check if the state has been modified
    ///
    /// # Returns
    ///
    /// True if the state has been modified, false otherwise
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
    
    /// Mark the state as clean
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }
    
    /// Get all keys in the state
    ///
    /// # Returns
    ///
    /// An iterator over all keys in the state
    pub fn keys(&self) -> impl Iterator<Item = &Vec<u8>> {
        self.inner.keys()
    }
    
    /// Get all keys with a given prefix
    ///
    /// # Arguments
    ///
    /// * `prefix` - The prefix to match
    ///
    /// # Returns
    ///
    /// An iterator over all keys with the given prefix
    pub fn keys_with_prefix<'a>(&'a self, prefix: &'a [u8]) -> impl Iterator<Item = &'a Vec<u8>> + 'a {
        self.inner.keys().filter(move |k| k.starts_with(prefix))
    }
    
    /// Get all key-value pairs in the state
    ///
    /// # Returns
    ///
    /// An iterator over all key-value pairs in the state
    pub fn iter(&self) -> impl Iterator<Item = (&Vec<u8>, &Vec<u8>)> {
        self.inner.iter()
    }
    
    /// Get the number of key-value pairs in the state
    ///
    /// # Returns
    ///
    /// The number of key-value pairs in the state
    pub fn len(&self) -> usize {
        self.inner.len()
    }
    
    /// Check if the state is empty
    ///
    /// # Returns
    ///
    /// True if the state is empty, false otherwise
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    
    /// Clear the state
    pub fn clear(&mut self) {
        if !self.inner.is_empty() {
            self.inner.clear();
            self.dirty = true;
        }
    }
}

/// Block metadata
///
/// Contains metadata about a block, including height, hash, and timestamp.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockMetadata {
    /// Block height
    pub height: u32,
    
    /// Block hash
    pub hash: String,
    
    /// Block timestamp (milliseconds since UNIX epoch)
    pub timestamp: u64,
}

/// Block cache entry
///
/// Represents an entry in the block cache, including block metadata,
/// state snapshot, and CDC messages.
#[derive(Debug, Clone)]
pub struct BlockCacheEntry {
    /// Block metadata
    pub metadata: BlockMetadata,
    
    /// State snapshot
    pub state_snapshot: TransformState,
    
    /// CDC messages generated for this block
    pub cdc_messages: Vec<CdcMessage>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cdc_message_serialization() {
        let message = CdcMessage {
            header: CdcHeader {
                source: "test_source".to_string(),
                timestamp: 1672531200000, // 2023-01-01T00:00:00Z in milliseconds
                block_height: 123456,
                block_hash: "000000000000000000024bead8df69990852c202db0e0097c1a12ea637d7e96d".to_string(),
                transaction_id: Some("tx123".to_string()),
            },
            payload: CdcPayload {
                operation: CdcOperation::Create,
                table: "test_table".to_string(),
                key: "test_key".to_string(),
                before: None,
                after: Some(serde_json::json!({
                    "field1": "value1",
                    "field2": 42
                })),
            },
        };
        
        let json = serde_json::to_string(&message).unwrap();
        let deserialized: CdcMessage = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized, message);
    }
    
    #[test]
    fn test_transform_state() {
        let mut state = TransformState::new();
        
        // Test empty state
        assert!(state.is_empty());
        assert_eq!(state.len(), 0);
        assert!(!state.is_dirty());
        
        // Test set and get
        let key = b"test_key".to_vec();
        let value = b"test_value".to_vec();
        state.set(key.clone(), value.clone());
        
        assert!(!state.is_empty());
        assert_eq!(state.len(), 1);
        assert!(state.is_dirty());
        assert_eq!(state.get(&key), Some(&value));
        
        // Test mark_clean
        state.mark_clean();
        assert!(!state.is_dirty());
        
        // Test delete
        assert!(state.delete(&key));
        assert!(state.is_empty());
        assert!(state.is_dirty());
        assert_eq!(state.get(&key), None);
        
        // Test keys_with_prefix
        state.set(b"prefix1_key1".to_vec(), b"value1".to_vec());
        state.set(b"prefix1_key2".to_vec(), b"value2".to_vec());
        state.set(b"prefix2_key1".to_vec(), b"value3".to_vec());
        
        let prefix1_keys: Vec<_> = state.keys_with_prefix(b"prefix1_").collect();
        assert_eq!(prefix1_keys.len(), 2);
        assert!(prefix1_keys.contains(&&b"prefix1_key1".to_vec()));
        assert!(prefix1_keys.contains(&&b"prefix1_key2".to_vec()));
        
        // Test clear
        state.clear();
        assert!(state.is_empty());
        assert!(state.is_dirty());
    }
}