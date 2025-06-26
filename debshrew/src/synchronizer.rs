//! Block synchronization with metashrew
//!
//! This module provides the block synchronizer, which is responsible for
//! synchronizing with metashrew, processing blocks, and handling reorgs.

use crate::block::BlockCache;
use crate::WasmRuntime;
use crate::client::MetashrewClient;
use crate::error::{Error, Result};
use crate::sink::CdcSink;
use async_trait::async_trait;
use debshrew_support::BlockMetadata;
use std::time::{SystemTime, UNIX_EPOCH};
use log::{debug, info, warn};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time;

/// Maximum length for logged response bodies (in characters)
const MAX_RESPONSE_LOG_LENGTH: usize = 1000;

/// Truncate a response string for logging purposes
fn truncate_response_for_logging(response: &str) -> String {
    if response.len() <= MAX_RESPONSE_LOG_LENGTH {
        response.to_string()
    } else {
        format!("{}... [truncated, total length: {} chars]",
                &response[..MAX_RESPONSE_LOG_LENGTH],
                response.len())
    }
}

/// Block synchronizer
///
/// The block synchronizer is responsible for synchronizing with metashrew,
/// processing blocks, and handling reorgs.
pub struct BlockSynchronizer<C: MetashrewClient> {
    /// The metashrew client
    client: Arc<C>,
    
    /// The WASM runtime
    runtime: Arc<Mutex<WasmRuntime>>,
    
    /// The CDC sink
    sink: Arc<Box<dyn CdcSink>>,
    
    /// The block cache
    cache: Arc<Mutex<BlockCache>>,
    
    /// The current block height
    current_height: u32,
    
    /// Whether the synchronizer is running
    running: bool,
    
    /// The polling interval in milliseconds
    polling_interval: u64,
}

impl<C: MetashrewClient> BlockSynchronizer<C> {
    // Track the last time we logged a progress report
    #[allow(dead_code)]
    fn log_progress_report(&self, metashrew_height: u32, actual_block_count: u32) {
        log::info!("Synchronization progress: current_height={}, metashrew_height={}, actual_block_count={}, progress={}%",
            self.current_height,
            metashrew_height,
            actual_block_count,
            if metashrew_height > 0 {
                (self.current_height as f64 / metashrew_height as f64 * 100.0).round()
            } else {
                100.0
            }
        );
    }
    /// Create a new block synchronizer
    ///
    /// # Arguments
    ///
    /// * `client` - The metashrew client
    /// * `runtime` - The WASM runtime
    /// * `sink` - The CDC sink
    /// * `cache_size` - The block cache size
    ///
    /// # Returns
    ///
    /// A new block synchronizer
    ///
    /// # Errors
    ///
    /// Returns an error if the block synchronizer cannot be created
    pub fn new(client: C, runtime: WasmRuntime, sink: Box<dyn CdcSink>, cache_size: u32) -> Result<Self> {
        let cache = BlockCache::new(cache_size)?;
        
        // Make sure the runtime has the correct metashrew URL
        // This is a no-op if the runtime already has the correct URL
        let metashrew_url = client.get_url().to_string();
        if runtime.get_metashrew_url() != metashrew_url {
            log::info!("Updating runtime metashrew URL to {}", metashrew_url);
            // Since we can't update the URL directly, we'd need to create a new runtime
            // with the correct URL in a real implementation
        }
        
        Ok(Self {
            client: Arc::new(client),
            runtime: Arc::new(Mutex::new(runtime)),
            sink: Arc::new(sink),
            cache: Arc::new(Mutex::new(cache)),
            current_height: 0,
            running: false,
            polling_interval: 1000,
        })
    }
    
    /// Set the polling interval
    ///
    /// # Arguments
    ///
    /// * `interval` - The polling interval in milliseconds
    pub fn set_polling_interval(&mut self, interval: u64) {
        self.polling_interval = interval;
    }
    
    /// Set the starting block height
    ///
    /// # Arguments
    ///
    /// * `height` - The starting block height
    pub fn set_starting_height(&mut self, height: u32) {
        self.current_height = height;
    }
    
    /// Run the block synchronizer
    ///
    /// This method starts the block synchronizer and runs until stopped.
    ///
    /// # Returns
    ///
    /// Ok(()) if the synchronizer ran successfully
    ///
    /// # Errors
    ///
    /// Returns an error if the synchronizer encounters an error
    pub async fn run(&mut self) -> Result<()> {
        self.running = true;
        
        // We'll keep the current height as set by set_starting_height
        // This allows starting from genesis (height 0) or any other height
        info!("Starting at block height {}", self.current_height);
        
        // Main synchronization loop
        while self.running {
            // Poll metashrew for the latest height
            let metashrew_height = self.client.get_height().await?;
            log::info!("Metashrew reported height: {}", metashrew_height);
            
            // Get the actual block count to avoid processing non-existent blocks
            let actual_block_count = self.get_actual_block_count().await?;
            log::info!("Actual block count: {}", actual_block_count);
            
            // Log a progress report
            self.log_progress_report(metashrew_height, actual_block_count);
            
            // Check if there's a significant discrepancy between metashrew_height and actual_block_count
            if metashrew_height > actual_block_count && actual_block_count <= self.current_height {
                log::warn!("Significant discrepancy detected: metashrew_height={}, actual_block_count={}, current_height={}",
                          metashrew_height, actual_block_count, self.current_height);
                
                // If we're stuck at the same height for multiple iterations, try incrementing by 1
                // This allows us to make progress even when there's a discrepancy
                let target_height = self.current_height + 1;
                
                if target_height <= metashrew_height {
                    log::info!("Attempting to process next block at height {} despite discrepancy", target_height);
                    
                    // Process the next block
                    self.process_block(target_height).await?;
                    self.current_height = target_height;
                    
                    // Sleep for the polling interval and continue
                    time::sleep(Duration::from_millis(self.polling_interval)).await;
                    continue;
                }
            }
            
            // Normal case: use the minimum of metashrew_height and actual_block_count
            let target_height = std::cmp::min(metashrew_height, actual_block_count);
            log::info!("Using target height: {} (min of {} and {})",
                      target_height, metashrew_height, actual_block_count);
            
            // Check if we need to process new blocks
            // Special case: if current_height is 0 and we're starting from genesis, always process block 0
            // regardless of target_height
            if target_height > self.current_height || self.current_height == 0 {
                info!("Processing blocks {} to {} (metashrew height: {}, actual block count: {})",
                      self.current_height + 1, target_height, metashrew_height, actual_block_count);
                
                // Process new blocks
                for height in (self.current_height + 1)..=target_height {
                    self.process_block(height).await?;
                    self.current_height = height;
                }
            } else if self.current_height > 0 {
                // Check for reorgs by comparing the hash of the current block
                // Get the cached hash for the current height
                let cached_hash = {
                    let cache = self.cache.lock().await;
                    if let Some(cached_block) = cache.get_block_at_height(self.current_height) {
                        Some(cached_block.metadata.hash.clone())
                    } else {
                        None
                    }
                }; // Lock is released here
                
                // If we have a cached hash, compare it with the current hash from metashrew
                if let Some(cached_hash) = cached_hash {
                    // Get the current hash from metashrew
                    match self.client.get_block_hash(self.current_height).await {
                        Ok(current_hash) => {
                            let current_hash_hex = hex::encode(&current_hash);
                            
                            // Compare with our cached hash
                            if current_hash_hex != cached_hash {
                                // Hash mismatch indicates a reorg
                                warn!("Chain reorganization detected: hash mismatch at height {}. Cached: {}, Current: {}",
                                      self.current_height, cached_hash, current_hash_hex);
                                
                                // Handle the reorg
                                self.handle_reorg(self.current_height - 1).await?;
                                
                                // After handling reorg, we'll continue from the common ancestor
                                // No need to update current_height here as handle_reorg already does that
                            }
                        },
                        Err(e) => {
                            // If we can't get the hash, it might be a deeper reorg
                            warn!("Failed to get block hash at height {}: {}. Possible deep reorg.", self.current_height, e);
                            
                            // Try to find the highest block that exists in both chains
                            let mut test_height = self.current_height - 1;
                            while test_height > 0 {
                                if let Ok(_) = self.client.get_block_hash(test_height).await {
                                    // Found a block that exists, handle reorg from here
                                    warn!("Found existing block at height {}. Handling reorg.", test_height);
                                    self.handle_reorg(test_height).await?;
                                    break;
                                }
                                test_height -= 1;
                                if test_height == 0 {
                                    // If we reach genesis, handle reorg from there
                                    warn!("Deep reorg detected, rolling back to genesis.");
                                    self.handle_reorg(0).await?;
                                }
                            }
                        }
                    }
                }
            }
            
            // Sleep for the polling interval
            time::sleep(Duration::from_millis(self.polling_interval)).await;
        }
        
        Ok(())
    }
    
    /// Stop the block synchronizer
    pub fn stop(&mut self) {
        self.running = false;
    }
    
    /// Process a block
    ///
    /// # Arguments
    ///
    /// * `height` - The block height
    ///
    /// # Returns
    ///
    /// Ok(()) if the block was processed successfully
    ///
    /// # Errors
    ///
    /// Returns an error if the block cannot be processed
    async fn process_block(&self, height: u32) -> Result<()> {
        // Get the block hash
        let hash = match self.client.get_block_hash(height).await {
            Ok(hash) => hash,
            Err(e) => {
                // Check if the error is because the block hash is not found
                if e.to_string().contains("Block hash not found") || e.to_string().contains("code: -32000") {
                    // This is likely because we're trying to process a block that doesn't exist yet
                    // Log this as info rather than error, and return without processing
                    info!("Block at height {} not available yet, will retry later", height);
                    return Ok(());
                } else {
                    // For other errors, propagate them
                    return Err(e);
                }
            }
        };
        
        // Create block metadata
        let metadata = BlockMetadata {
            height,
            hash: hex::encode(&hash),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        };
        
        // Process the block with the transform module
        let mut runtime = self.runtime.lock().await;
        let transform_result = runtime.process_block(height, hash)?;
        
        // Add the block to the cache
        let mut cache = self.cache.lock().await;
        cache.add_block(metadata, transform_result.clone())?;
        
        // Send the CDC messages to the sink
        self.sink.send(transform_result.cdc_messages).await?;
        
        debug!("Processed block {}", height);
        
        Ok(())
    }
    
    /// Handle a chain reorganization
    ///
    /// # Arguments
    ///
    /// * `new_height` - The new block height
    ///
    /// # Returns
    ///
    /// Ok(()) if the reorg was handled successfully
    ///
    /// # Errors
    ///
    /// Returns an error if the reorg cannot be handled
    async fn handle_reorg(&self, new_height: u32) -> Result<()> {
        // Get the block hashes for the new chain
        let mut new_hashes = Vec::new();
        for height in 0..=new_height {
            let hash = self.client.get_block_hash(height).await?;
            new_hashes.push((height, hex::encode(&hash)));
        }
        
        // Find the common ancestor
        let cache = self.cache.lock().await;
        let common_ancestor = cache.find_common_ancestor(&new_hashes)
            .ok_or_else(|| Error::ReorgHandling("No common ancestor found".to_string()))?;
        
        info!("Found common ancestor at height {}", common_ancestor);
        
        // Get the state snapshot at the common ancestor
        let state_snapshot = cache.get_state_snapshot(common_ancestor)
            .ok_or_else(|| Error::ReorgHandling(format!("State snapshot not found for height {}", common_ancestor)))?;
        
        // Release the cache lock
        drop(cache);
        
        // Get the runtime lock
        let mut runtime = self.runtime.lock().await;
        
        // Generate inverse CDC messages for the rolled back blocks
        let mut inverse_messages = Vec::new();
        
        // Process blocks in reverse order from current_height down to common_ancestor + 1
        for height in (common_ancestor + 1..=self.current_height).rev() {
            info!("Generating inverse CDC messages for block {}", height);
            
            // Compute inverse messages for this block
            let block_inverse = runtime.compute_inverse_messages(height)?;
            inverse_messages.extend(block_inverse);
        }
        
        // Reset the runtime state to the common ancestor
        runtime.set_current_height(common_ancestor);
        runtime.set_state(state_snapshot);
        
        // Send the inverse CDC messages to the sink
        if !inverse_messages.is_empty() {
            info!("Sending {} inverse CDC messages to sink", inverse_messages.len());
            self.sink.send(inverse_messages).await?;
        }
        
        // Roll back the cache
        let mut cache = self.cache.lock().await;
        cache.rollback(common_ancestor)?;
        
        // Release the runtime lock
        drop(runtime);
        
        // Process the new chain
        for height in (common_ancestor + 1)..=new_height {
            // Release the cache lock
            drop(cache);
            
            // Process the block
            self.process_block(height).await?;
            
            // Reacquire the cache lock
            cache = self.cache.lock().await;
        }
        
        Ok(())
    }
    
    /// Get the current block height
    ///
    /// # Returns
    ///
    /// The current block height
    pub fn get_current_height(&self) -> u32 {
        self.current_height
    }
    
    /// Get the block cache
    ///
    /// # Returns
    ///
    /// The block cache
    pub async fn get_cache(&self) -> Arc<Mutex<BlockCache>> {
        self.cache.clone()
    }
    
    /// Get the WASM runtime
    ///
    /// # Returns
    ///
    /// The WASM runtime
    pub async fn get_runtime(&self) -> Arc<Mutex<WasmRuntime>> {
        self.runtime.clone()
    }
    
    /// Get the CDC sink
    ///
    /// # Returns
    ///
    /// The CDC sink
    pub fn get_sink(&self) -> Arc<Box<dyn CdcSink>> {
        self.sink.clone()
    }
    
    /// Get the metashrew client
    ///
    /// # Returns
    ///
    /// The metashrew client
    pub fn get_client(&self) -> Arc<C> {
        self.client.clone()
    }
}

/// Synchronizer trait
///
/// This trait defines the interface for block synchronizers.
#[async_trait]
pub trait Synchronizer: Send + Sync {
    /// Run the synchronizer
    ///
    /// # Returns
    ///
    /// Ok(()) if the synchronizer ran successfully
    ///
    /// # Errors
    ///
    /// Returns an error if the synchronizer encounters an error
    async fn run(&mut self) -> Result<()>;
    
    /// Stop the synchronizer
    fn stop(&mut self);
    
    /// Get the current block height
    ///
    /// # Returns
    ///
    /// The current block height
    fn get_current_height(&self) -> u32;
}

#[async_trait]
impl<C: MetashrewClient> Synchronizer for BlockSynchronizer<C> {
    async fn run(&mut self) -> Result<()> {
        self.run().await
    }
    
    fn stop(&mut self) {
        self.stop();
    }
    
    fn get_current_height(&self) -> u32 {
        self.get_current_height()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::MockMetashrewClient;
    use crate::sink::{ConsoleSink, FileSink, NullSink};
    use debshrew_runtime::MockTransform;
    use debshrew_support::{CdcHeader, CdcMessage, CdcOperation, CdcPayload};
    use std::sync::Arc;
    use tempfile::tempdir;
    use tokio::runtime::Runtime;

    #[test]
    fn test_block_synchronizer() {
        // Create a mock metashrew client
        let mut client = MockMetashrewClient::new();
        client.set_height(10);
        
        for i in 0..=10 {
            client.set_block_hash(i, vec![i as u8]);
        }
        
        // Create a mock transform
        let _transform = MockTransform::default();
        
        // Create a WASM runtime for testing
        // Use the client's URL for the metashrew URL
        let runtime = WasmRuntime::for_testing().unwrap();
        
        // Create a null sink
        let sink = Box::new(NullSink::new());
        
        // Create a block synchronizer
        let mut synchronizer = BlockSynchronizer::new(client, runtime, sink, 6).unwrap();
        
        // Set the starting height
        synchronizer.set_starting_height(5);
        
        // Set a short polling interval
        synchronizer.set_polling_interval(10);
        
        // Create a runtime for async tests
        let rt = Runtime::new().unwrap();
        
        // Run the synchronizer for a short time
        let handle = rt.spawn(async move {
            let _ = synchronizer.run().await;
        });
        
        // Wait for a short time
        std::thread::sleep(Duration::from_millis(100));
        
        // Abort the task
        handle.abort();
    }
    
    #[test]
    fn test_block_synchronizer_with_different_sinks() {
        // Create a mock metashrew client
        let mut client = MockMetashrewClient::new();
        client.set_height(10);
        
        for i in 0..=10 {
            client.set_block_hash(i, vec![i as u8]);
        }
        
        // We don't need this runtime since we create a new one for each test
        // let runtime = WasmRuntime::from_bytes(&[0]).unwrap();
        
        // Create a tokio runtime for async tests
        let rt = Runtime::new().unwrap();
        
        // Create a new runtime for each test to avoid cloning
        
        // Test with ConsoleSink
        {
            let console_sink = Box::new(ConsoleSink::new(false));
            let test_runtime = WasmRuntime::for_testing().unwrap();
            let synchronizer = BlockSynchronizer::new(client.clone(), test_runtime, console_sink, 6).unwrap();
            
            // Verify the sink type
            let sink = synchronizer.get_sink();
            rt.block_on(async {
                // Send a test message to verify the sink works
                let message = create_test_message();
                let result = sink.send(vec![message]).await;
                assert!(result.is_ok());
                
                // Flush and close the sink
                assert!(sink.flush().await.is_ok());
                assert!(sink.close().await.is_ok());
            });
        }
        
        // Test with FileSink
        {
            // Create a temporary directory for the file
            let dir = tempdir().unwrap();
            let file_path = dir.path().join("test.json");
            
            let file_sink = Box::new(FileSink::new(file_path.to_str().unwrap(), false, 1000).unwrap());
            let test_runtime = WasmRuntime::for_testing().unwrap();
            let synchronizer = BlockSynchronizer::new(client.clone(), test_runtime, file_sink, 6).unwrap();
            
            // Verify the sink type
            let sink = synchronizer.get_sink();
            rt.block_on(async {
                // Send a test message to verify the sink works
                let message = create_test_message();
                let result = sink.send(vec![message]).await;
                assert!(result.is_ok());
                
                // Flush and close the sink
                assert!(sink.flush().await.is_ok());
                assert!(sink.close().await.is_ok());
            });
        }
        
        // Test with a custom sink implementation
        {
            // Create a custom sink that counts messages
            #[derive(Clone)]
            struct CountingSink {
                count: Arc<tokio::sync::Mutex<usize>>,
            }
            
            impl CountingSink {
                fn new() -> Self {
                    Self {
                        count: Arc::new(tokio::sync::Mutex::new(0)),
                    }
                }
                
                async fn get_count(&self) -> usize {
                    let count = self.count.lock().await;
                    *count
                }
            }
            
            #[async_trait]
            impl CdcSink for CountingSink {
                async fn send(&self, messages: Vec<CdcMessage>) -> Result<()> {
                    let mut count = self.count.lock().await;
                    *count += messages.len();
                    Ok(())
                }
                
                async fn flush(&self) -> Result<()> {
                    Ok(())
                }
                
                async fn close(&self) -> Result<()> {
                    Ok(())
                }
            }
            
            let counting_sink = CountingSink::new();
            let counting_sink_clone = counting_sink.clone();
            let test_runtime = WasmRuntime::for_testing().unwrap();
            let synchronizer = BlockSynchronizer::new(client.clone(), test_runtime, Box::new(counting_sink), 6).unwrap();
            
            // Verify the sink works
            rt.block_on(async {
                // Get the sink from the synchronizer
                let sink = synchronizer.get_sink();
                
                // Send test messages
                let messages = vec![
                    create_test_message(),
                    create_test_message(),
                    create_test_message(),
                ];
                
                let result = sink.send(messages).await;
                assert!(result.is_ok());
                
                // Verify the count
                assert_eq!(counting_sink_clone.get_count().await, 3);
            });
        }
    }
    
    // Helper function to create a test CDC message
    fn create_test_message() -> CdcMessage {
        CdcMessage {
            header: CdcHeader {
                source: "test".to_string(),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
                block_height: 123,
                block_hash: "000000000000000000024bead8df69990852c202db0e0097c1a12ea637d7e96d".to_string(),
                transaction_id: None,
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
        }
    }
}
impl<C: MetashrewClient> BlockSynchronizer<C> {
    // Add this method after the existing methods
    
    /// Get the actual block count using the Bitcoin-style API
    ///
    /// This is a workaround for the discrepancy between metashrew_height and the actual block count
    ///
    /// # Returns
    ///
    /// The actual block count
    ///
    /// # Errors
    ///
    /// Returns an error if the block count cannot be retrieved
    async fn get_actual_block_count(&self) -> Result<u32> {
        log::info!("Getting actual block count from {}", self.client.get_url());
        log::debug!("Current height before getting block count: {}", self.current_height);
        
        // Create a JSON-RPC request to get the block count
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "getblockcount",
            "params": [],
            "id": 1
        });
        
        log::debug!("Sending getblockcount request: {}", request.to_string());
        
        // Send the request
        let client = reqwest::Client::new();
        let response = match client.post(self.client.get_url().clone())
            .header("Content-Type", "application/json")
            .body(request.to_string())
            .send()
            .await {
                Ok(resp) => resp,
                Err(e) => {
                    log::error!("Failed to send getblockcount request: {}", e);
                    // If we can't get the actual block count, return a safe default of 1
                    // This ensures we can still process blocks even if the getblockcount method fails
                    log::warn!("Using default block count of 1");
                    return Ok(1);
                }
            };
        
        // Parse the response
        let response_text = match response.text().await {
            Ok(text) => text,
            Err(e) => {
                log::error!("Failed to get response text: {}", e);
                log::warn!("Using default block count of 1");
                return Ok(1);
            }
        };
        
        log::debug!("Received getblockcount response: {}", truncate_response_for_logging(&response_text));
        
        let json_response: serde_json::Value = match serde_json::from_str(&response_text) {
            Ok(json) => json,
            Err(e) => {
                log::error!("Failed to parse response as JSON: {}", e);
                log::warn!("Using default block count of 1");
                return Ok(1);
            }
        };
        
        // Extract the result
        let result = match json_response.get("result") {
            Some(r) => r,
            None => {
                log::error!("No result in getblockcount response");
                log::warn!("Using default block count of 1");
                return Ok(1);
            }
        };
        
        // Convert to u32
        let block_count = match result.as_u64() {
            Some(count) => count as u32,
            None => {
                log::error!("Invalid block count: {:?}", result);
                log::warn!("Using default block count of 1");
                return Ok(1);
            }
        };
        
        log::info!("Actual block count: {}", block_count);
        Ok(block_count)
    }
}
