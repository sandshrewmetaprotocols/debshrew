//! WASM runtime implementation for debshrew
//!
//! This module provides the WASM runtime implementation for debshrew,
//! including loading and executing WASM modules, providing host functions,
//! and managing WASM memory.

use crate::error::Result;
use debshrew_runtime::transform::TransformResult;
use debshrew_support::{CdcMessage, CdcHeader, CdcOperation, CdcPayload, TransformState};
use std::collections::HashMap;
use std::path::Path;
use wasmtime::{Engine, Module, Store, Linker};
use anyhow::anyhow;
use std::time::{SystemTime, UNIX_EPOCH};

/// WASM runtime for executing transform modules
pub struct WasmRuntime {
    /// The wasmtime engine
    engine: Engine,
    
    /// The WASM module
    module: Module,
    
    /// The current block height
    current_height: u32,
    
    /// The current block hash
    current_hash: Vec<u8>,
    
    /// The transform state
    state: TransformState,
    
    /// Cache of CDC messages by block height
    cdc_cache: HashMap<u32, Vec<CdcMessage>>,
}

impl std::fmt::Debug for WasmRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmRuntime")
            .field("current_height", &self.current_height)
            .field("current_hash", &self.current_hash)
            .field("state", &self.state)
            .field("cdc_cache", &self.cdc_cache.keys())
            .finish_non_exhaustive()
    }
}

impl WasmRuntime {
    /// Create a new WASM runtime
    ///
    /// # Arguments
    ///
    /// * `wasm_path` - The path to the WASM module
    ///
    /// # Returns
    ///
    /// A new WASM runtime
    ///
    /// # Errors
    ///
    /// Returns an error if the WASM module cannot be loaded
    pub fn new<P: AsRef<Path>>(wasm_path: P) -> Result<Self> {
        let engine = Engine::default();
        let module = Module::from_file(&engine, wasm_path)
            .map_err(|e| anyhow!("Failed to load WASM module: {}", e))?;

        Ok(Self {
            engine,
            module,
            current_height: 0,
            current_hash: Vec::new(),
            state: TransformState::new(),
            cdc_cache: HashMap::new(),
        })
    }

    /// Create a new WASM runtime from WASM bytes
    ///
    /// # Arguments
    ///
    /// * `wasm_bytes` - The WASM module bytes
    ///
    /// # Returns
    ///
    /// A new WASM runtime
    ///
    /// # Errors
    ///
    /// Returns an error if the WASM module cannot be loaded
    pub fn from_bytes(wasm_bytes: &[u8]) -> Result<Self> {
        let engine = Engine::default();
        let module = Module::from_binary(&engine, wasm_bytes)
            .map_err(|e| anyhow!("Failed to load WASM module from bytes: {}", e))?;

        Ok(Self {
            engine,
            module,
            current_height: 0,
            current_hash: Vec::new(),
            state: TransformState::new(),
            cdc_cache: HashMap::new(),
        })
    }

    /// Set the current block height
    ///
    /// # Arguments
    ///
    /// * `height` - The current block height
    pub fn set_current_height(&mut self, height: u32) {
        self.current_height = height;
    }

    /// Set the current block hash
    ///
    /// # Arguments
    ///
    /// * `hash` - The current block hash
    pub fn set_current_hash(&mut self, hash: Vec<u8>) {
        self.current_hash = hash;
    }

    /// Set the transform state
    ///
    /// # Arguments
    ///
    /// * `state` - The transform state
    pub fn set_state(&mut self, state: TransformState) {
        self.state = state;
    }

    /// Get the transform state
    ///
    /// # Returns
    ///
    /// The transform state
    pub fn get_state(&self) -> TransformState {
        self.state.clone()
    }
    
    /// Process a block
    ///
    /// # Arguments
    ///
    /// * `height` - The block height
    /// * `hash` - The block hash
    ///
    /// # Returns
    ///
    /// The result of processing the block, including CDC messages and a state snapshot
    ///
    /// # Errors
    ///
    /// Returns an error if block processing fails
    pub fn process_block(&mut self, height: u32, hash: Vec<u8>) -> Result<TransformResult> {
        // Set the current block height and hash
        self.set_current_height(height);
        self.set_current_hash(hash);
        
        // Create a new store with our runtime data
        let mut store = Store::new(&self.engine, ());
        
        // Define host functions that will be imported by the WASM module
        let mut linker = Linker::new(&self.engine);
        
        // Define the "env" module and its functions
        let env_module = "env";
        
        // Create shared state for closures
        let current_height = self.current_height;
        let current_hash_for_block_hash = self.current_hash.clone();
        
        // Register all required host functions
        linker.func_wrap(env_module, "__load", |_ptr: i32| {
            // Simple stub implementation
        }).map_err(|e| anyhow!("Failed to register __load: {}", e))?;
        
        linker.func_wrap(env_module, "__view", |_view_name_ptr: i32, _input_ptr: i32| -> i32 {
            // Simple stub implementation - return 0 for success
            0
        }).map_err(|e| anyhow!("Failed to register __view: {}", e))?;
        
        linker.func_wrap(env_module, "__stdout", |mut caller: wasmtime::Caller<'_, ()>, ptr: i32| {
            // Read the message from WASM memory
            let memory = match caller.get_export("memory") {
                Some(wasmtime::Extern::Memory(mem)) => mem,
                _ => return, // No memory export, can't read the message
            };
            
            // Read the length (first 4 bytes)
            let mut len_bytes = [0u8; 4];
            if memory.read(&caller, ptr as usize, &mut len_bytes).is_err() {
                return; // Failed to read length
            }
            let len = u32::from_le_bytes(len_bytes) as usize;
            
            // Read the message
            let mut message = vec![0u8; len];
            if memory.read(&caller, (ptr + 4) as usize, &mut message).is_err() {
                return; // Failed to read message
            }
            
            // Convert to string and log
            if let Ok(s) = std::str::from_utf8(&message) {
                log::info!("[WASM stdout] {}", s.trim_end());
            }
        }).map_err(|e| anyhow!("Failed to register __stdout: {}", e))?;
        
        linker.func_wrap(env_module, "__stderr", |mut caller: wasmtime::Caller<'_, ()>, ptr: i32| {
            // Read the message from WASM memory
            let memory = match caller.get_export("memory") {
                Some(wasmtime::Extern::Memory(mem)) => mem,
                _ => return, // No memory export, can't read the message
            };
            
            // Read the length (first 4 bytes)
            let mut len_bytes = [0u8; 4];
            if memory.read(&caller, ptr as usize, &mut len_bytes).is_err() {
                return; // Failed to read length
            }
            let len = u32::from_le_bytes(len_bytes) as usize;
            
            // Read the message
            let mut message = vec![0u8; len];
            if memory.read(&caller, (ptr + 4) as usize, &mut message).is_err() {
                return; // Failed to read message
            }
            
            // Convert to string and log
            if let Ok(s) = std::str::from_utf8(&message) {
                log::warn!("[WASM stderr] {}", s.trim_end());
            }
        }).map_err(|e| anyhow!("Failed to register __stderr: {}", e))?;
        
        linker.func_wrap(env_module, "__height", move || -> i32 {
            current_height as i32
        }).map_err(|e| anyhow!("Failed to register __height: {}", e))?;
        
        linker.func_wrap(env_module, "__block_hash", move || -> i32 {
            current_hash_for_block_hash.len() as i32
        }).map_err(|e| anyhow!("Failed to register __block_hash: {}", e))?;
        
        // We no longer need the __push_cdc_message host function as the WASM program
        // will return a pointer to the serialized CDC messages at the end of execution
        
        linker.func_wrap(env_module, "__get_state", |_key: i32| -> i32 {
            0
        }).map_err(|e| anyhow!("Failed to register __get_state: {}", e))?;
        
        linker.func_wrap(env_module, "__set_state", |_key: i32, _value: i32| -> i32 {
            0
        }).map_err(|e| anyhow!("Failed to register __set_state: {}", e))?;
        
        linker.func_wrap(env_module, "__delete_state", |_key: i32| -> i32 {
            0
        }).map_err(|e| anyhow!("Failed to register __delete_state: {}", e))?;
        
        // Create a new instance with the imported host functions
        let instance = linker.instantiate(&mut store, &self.module)
            .map_err(|e| anyhow!("Failed to instantiate WASM module: {}", e))?;
        
        // Get the process_block function
        let process_block = instance.get_typed_func::<(), i32>(&mut store, "process_block")
            .map_err(|e| anyhow!("Failed to get process_block function: {}", e))?;
        
        // Call the process_block function
        // The return value is a pointer to the serialized CDC messages
        let cdc_ptr = process_block.call(&mut store, ())
            .map_err(|e| anyhow!("Failed to call process_block function: {}", e))?;
        
        if cdc_ptr < 0 {
            return Err(anyhow!("Process block failed with code {}", cdc_ptr).into());
        }
        
        // In a real implementation, we would deserialize the CDC messages from WASM memory
        // using the pointer returned by the process_block function
        // For now, we'll just create a simple CDC message
        let cdc_messages = vec![CdcMessage {
            header: CdcHeader {
                source: "block_transform".to_string(),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
                block_height: current_height,
                block_hash: hex::encode(&self.current_hash),
                transaction_id: None,
            },
            payload: CdcPayload {
                operation: CdcOperation::Create,
                table: "blocks".to_string(),
                key: current_height.to_string(),
                before: None,
                after: Some(serde_json::json!({
                    "height": current_height,
                    "hash": hex::encode(&self.current_hash),
                    "timestamp": SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                })),
            },
        }];
        
        // Cache CDC messages for this block
        self.cdc_cache.insert(height, cdc_messages.clone());
        
        // Update state from WASM memory
        // In a real implementation, we would extract the state from WASM memory
        // For now, we'll just use the existing state
        
        Ok(TransformResult::new(cdc_messages, self.state.clone()))
    }
    
    /// Handle a rollback
    ///
    /// # Arguments
    ///
    /// * `height` - The height to roll back to
    /// * `hash` - The hash to roll back to
    ///
    /// # Returns
    ///
    /// The result of the rollback, including CDC messages and a state snapshot
    ///
    /// # Errors
    ///
    /// Returns an error if the rollback fails
    pub fn rollback(&mut self, height: u32, hash: Vec<u8>) -> Result<TransformResult> {
        // Set the current block height and hash
        self.set_current_height(height);
        self.set_current_hash(hash);
        
        // Create a new store with our runtime data
        let mut store = Store::new(&self.engine, ());
        
        // Define host functions that will be imported by the WASM module
        let mut linker = Linker::new(&self.engine);
        
        // Define the "env" module and its functions
        let env_module = "env";
        
        // Create shared state for closures
        let current_height = self.current_height;
        let current_hash_for_block_hash = self.current_hash.clone();
        
        // Register all required host functions
        linker.func_wrap(env_module, "__load", |_ptr: i32| {
            // Simple stub implementation
        }).map_err(|e| anyhow!("Failed to register __load: {}", e))?;
        
        linker.func_wrap(env_module, "__view", |_view_name_ptr: i32, _input_ptr: i32| -> i32 {
            // Simple stub implementation - return 0 for success
            0
        }).map_err(|e| anyhow!("Failed to register __view: {}", e))?;
        
        linker.func_wrap(env_module, "__stdout", |_ptr: i32| {
            // Simple stub implementation
        }).map_err(|e| anyhow!("Failed to register __stdout: {}", e))?;
        
        linker.func_wrap(env_module, "__stderr", |_ptr: i32| {
            // Simple stub implementation
        }).map_err(|e| anyhow!("Failed to register __stderr: {}", e))?;
        
        linker.func_wrap(env_module, "__height", move || -> i32 {
            current_height as i32
        }).map_err(|e| anyhow!("Failed to register __height: {}", e))?;
        
        linker.func_wrap(env_module, "__block_hash", move || -> i32 {
            current_hash_for_block_hash.len() as i32
        }).map_err(|e| anyhow!("Failed to register __block_hash: {}", e))?;
        
        // We no longer need the __push_cdc_message host function as the WASM program
        // will return a pointer to the serialized CDC messages at the end of execution
        
        linker.func_wrap(env_module, "__get_state", |_key: i32| -> i32 {
            0
        }).map_err(|e| anyhow!("Failed to register __get_state: {}", e))?;
        
        linker.func_wrap(env_module, "__set_state", |_key: i32, _value: i32| -> i32 {
            0
        }).map_err(|e| anyhow!("Failed to register __set_state: {}", e))?;
        
        linker.func_wrap(env_module, "__delete_state", |_key: i32| -> i32 {
            0
        }).map_err(|e| anyhow!("Failed to register __delete_state: {}", e))?;
        
        // Create a new instance with the imported host functions
        let instance = linker.instantiate(&mut store, &self.module)
            .map_err(|e| anyhow!("Failed to instantiate WASM module: {}", e))?;
        
        // Get the rollback function
        let rollback = instance.get_typed_func::<(), i32>(&mut store, "rollback")
            .map_err(|e| anyhow!("Failed to get rollback function: {}", e))?;
        
        // Call the rollback function
        // The return value is a pointer to the serialized CDC messages
        let cdc_ptr = rollback.call(&mut store, ())
            .map_err(|e| anyhow!("Failed to call rollback function: {}", e))?;
        
        if cdc_ptr < 0 {
            return Err(anyhow!("Rollback failed with code {}", cdc_ptr).into());
        }
        
        // In a real implementation, we would deserialize the CDC messages from WASM memory
        // using the pointer returned by the rollback function
        // For now, we'll just create a simple CDC message
        let cdc_messages = vec![CdcMessage {
            header: CdcHeader {
                source: "block_transform".to_string(),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
                block_height: current_height,
                block_hash: hex::encode(&self.current_hash),
                transaction_id: None,
            },
            payload: CdcPayload {
                operation: CdcOperation::Delete,
                table: "blocks".to_string(),
                key: current_height.to_string(),
                before: Some(serde_json::json!({
                    "height": current_height,
                    "hash": hex::encode(&self.current_hash),
                    "timestamp": SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                })),
                after: None,
            },
        }];
        
        // Update state from WASM memory
        // In a real implementation, we would extract the state from WASM memory
        // For now, we'll just use the existing state
        
        Ok(TransformResult::new(cdc_messages, self.state.clone()))
    }
    
    /// Compute inverse CDC messages for a block
    ///
    /// # Arguments
    ///
    /// * `height` - The block height
    ///
    /// # Returns
    ///
    /// The inverse CDC messages
    ///
    /// # Errors
    ///
    /// Returns an error if the inverse messages cannot be computed
    pub fn compute_inverse_messages(&self, height: u32) -> Result<Vec<CdcMessage>> {
        if let Some(messages) = self.cdc_cache.get(&height) {
            let mut inverse = Vec::new();
            
            // Process messages in reverse order
            for message in messages.iter().rev() {
                let inverse_message = self.invert_cdc_message(message, height - 1)?;
                inverse.push(inverse_message);
            }
            
            Ok(inverse)
        } else {
            Err(anyhow!("No CDC messages found for block {}", height).into())
        }
    }
    
    /// Invert a CDC message
    ///
    /// # Arguments
    ///
    /// * `message` - The CDC message to invert
    /// * `new_height` - The new block height
    ///
    /// # Returns
    ///
    /// The inverted CDC message
    ///
    /// # Errors
    ///
    /// Returns an error if the message cannot be inverted
    fn invert_cdc_message(&self, message: &CdcMessage, new_height: u32) -> Result<CdcMessage> {
        let (operation, before, after) = match message.payload.operation {
            CdcOperation::Create => (
                CdcOperation::Delete,
                message.payload.after.clone(),
                None
            ),
            CdcOperation::Update => (
                CdcOperation::Update,
                message.payload.after.clone(),
                message.payload.before.clone()
            ),
            CdcOperation::Delete => (
                CdcOperation::Create,
                None,
                message.payload.before.clone()
            ),
        };
        
        Ok(CdcMessage {
            header: CdcHeader {
                source: message.header.source.clone(),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
                block_height: new_height,
                block_hash: hex::encode(&self.current_hash),
                transaction_id: None,
            },
            payload: CdcPayload {
                operation,
                table: message.payload.table.clone(),
                key: message.payload.key.clone(),
                before,
                after,
            },
        })
    }
    
    // The push_cdc_message method is no longer needed as we're now returning
    // CDC messages from the WASM program at the end of execution
    
    /// Register a view function
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the view function
    /// * `func` - The view function implementation
    pub fn register_view_function(
        &self,
        _name: &str,
        _func: Box<dyn Fn(&[u8]) -> Result<Vec<u8>> + Send>,
    ) {
        // In a real implementation, we would register the view function
        // For now, this is a stub
    }
    
    /// Create a mock WasmRuntime for testing
    ///
    /// # Returns
    ///
    /// A new WasmRuntime for testing
    ///
    /// # Errors
    ///
    /// Returns an error if the WASM module cannot be created
    #[cfg(any(test, feature = "testing"))]
    pub fn for_testing() -> Result<Self> {
        use wat::parse_str;
        
        // Create a simple WASM module
        let wasm_bytes = parse_str(
            r#"
            (module
                (func $process_block (export "process_block") (result i32)
                    i32.const 0
                )
                (func $rollback (export "rollback") (result i32)
                    i32.const 0
                )
                (memory (export "memory") 1)
            )
            "#,
        )
        .map_err(|e| anyhow!("Failed to create test WASM module: {}", e))?;
        
        Self::from_bytes(&wasm_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_invert_cdc_message() {
        let runtime = WasmRuntime::for_testing().unwrap();
        
        // Test inverting a Create message
        let create_message = CdcMessage {
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
        };
        
        let inverse = runtime.invert_cdc_message(&create_message, 122).unwrap();
        
        assert_eq!(inverse.payload.operation, CdcOperation::Delete);
        assert_eq!(inverse.payload.table, "test_table");
        assert_eq!(inverse.payload.key, "test_key");
        assert_eq!(inverse.payload.before, create_message.payload.after);
        assert_eq!(inverse.payload.after, None);
    }
}