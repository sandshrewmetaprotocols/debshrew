//! WASM runtime implementation for debshrew
//!
//! This module provides the WASM runtime implementation for debshrew,
//! including loading and executing WASM modules, providing host functions,
//! and managing WASM memory.

#[cfg(feature = "host")]
use crate::error::Result;
#[cfg(feature = "host")]
use crate::transform::TransformResult;
#[cfg(feature = "host")]
use debshrew_support::{CdcMessage, CdcHeader, CdcOperation, CdcPayload, TransformState};
#[cfg(feature = "host")]
use std::collections::HashMap;
#[cfg(feature = "host")]
use std::path::Path;
#[cfg(feature = "host")]
use wasmtime::{Engine, Instance, Module, Store};
#[cfg(feature = "host")]
use anyhow::anyhow;
#[cfg(feature = "host")]
use std::time::{SystemTime, UNIX_EPOCH};

/// WASM runtime for executing transform modules
#[cfg(feature = "host")]
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
    
    /// Buffer for CDC messages from the current operation
    cdc_messages: Vec<CdcMessage>,
}

#[cfg(feature = "host")]
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

#[cfg(feature = "host")]
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
            cdc_messages: Vec::new(),
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
            cdc_messages: Vec::new(),
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
        
        // Clear CDC message buffer
        self.cdc_messages.clear();
        
        // Create a new store with our runtime data
        // Use a reborrow of self to avoid moving it
        let mut store = Store::new(&self.engine, ());
        
        // Define host functions that will be imported by the WASM module
        let mut linker = wasmtime::Linker::new(&self.engine);
        
        // Define the "env" module and its functions
        let env_module = "env";
        
        // Register the host functions
        // Note: This is a simplified implementation. In a real implementation,
        // you would need to properly implement these functions to interact with
        // the host environment and the WASM memory.
        
        // For now, we'll just register stub functions that return 0
        // This should allow the WASM module to instantiate without errors
        
        // Define a macro to help register functions
        macro_rules! register_func {
            ($linker:expr, $module:expr, $name:expr, $func:expr) => {
                $linker.func_wrap($module, $name, $func)
                    .map_err(|e| anyhow!("Failed to register function {}: {}", $name, e))?;
            };
        }
        
        // Register all the required functions
        register_func!(linker, env_module, "__load", |_: i32| {});
        register_func!(linker, env_module, "__view", |_: i32, _: i32| -> i32 { 0 });
        register_func!(linker, env_module, "__stdout", |_: i32| {});
        register_func!(linker, env_module, "__stderr", |_: i32| {});
        register_func!(linker, env_module, "__height", || -> i32 { 0 });
        register_func!(linker, env_module, "__block_hash", || -> i32 { 0 });
        register_func!(linker, env_module, "__push_cdc_message", |_: i32| -> i32 { 0 });
        register_func!(linker, env_module, "__get_state", |_: i32| -> i32 { 0 });
        register_func!(linker, env_module, "__set_state", |_: i32, _: i32| -> i32 { 0 });
        register_func!(linker, env_module, "__delete_state", |_: i32| -> i32 { 0 });
        
        // Create a new instance with the imported host functions
        let instance = linker.instantiate(&mut store, &self.module)
            .map_err(|e| anyhow!("Failed to instantiate WASM module: {}", e))?;
        
        // Get the process_block function
        let process_block = instance.get_typed_func::<(), i32>(&mut store, "process_block")
            .map_err(|e| anyhow!("Failed to get process_block function: {}", e))?;
        
        // Call the process_block function
        let result = process_block.call(&mut store, ())
            .map_err(|e| anyhow!("Failed to call process_block function: {}", e))?;
        
        if result < 0 {
            return Err(anyhow!("Process block failed with code {}", result));
        }
        
        // Get the CDC messages that were pushed
        let cdc_messages = self.cdc_messages.clone();
        
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
        
        // Clear CDC message buffer
        self.cdc_messages.clear();
        
        // Create a new store with our runtime data
        // Use a reborrow of self to avoid moving it
        let mut store = Store::new(&self.engine, ());
        
        // Define host functions that will be imported by the WASM module
        let mut linker = wasmtime::Linker::new(&self.engine);
        
        // Define the "env" module and its functions
        let env_module = "env";
        
        // Register the host functions using the same macro as in process_block
        macro_rules! register_func {
            ($linker:expr, $module:expr, $name:expr, $func:expr) => {
                $linker.func_wrap($module, $name, $func)
                    .map_err(|e| anyhow!("Failed to register function {}: {}", $name, e))?;
            };
        }
        
        // Register all the required functions
        register_func!(linker, env_module, "__load", |_: i32| {});
        register_func!(linker, env_module, "__view", |_: i32, _: i32| -> i32 { 0 });
        register_func!(linker, env_module, "__stdout", |_: i32| {});
        register_func!(linker, env_module, "__stderr", |_: i32| {});
        register_func!(linker, env_module, "__height", || -> i32 { 0 });
        register_func!(linker, env_module, "__block_hash", || -> i32 { 0 });
        register_func!(linker, env_module, "__push_cdc_message", |_: i32| -> i32 { 0 });
        register_func!(linker, env_module, "__get_state", |_: i32| -> i32 { 0 });
        register_func!(linker, env_module, "__set_state", |_: i32, _: i32| -> i32 { 0 });
        register_func!(linker, env_module, "__delete_state", |_: i32| -> i32 { 0 });
        
        // Create a new instance with the imported host functions
        let instance = linker.instantiate(&mut store, &self.module)
            .map_err(|e| anyhow!("Failed to instantiate WASM module: {}", e))?;
        
        // Get the rollback function
        let rollback = instance.get_typed_func::<(), i32>(&mut store, "rollback")
            .map_err(|e| anyhow!("Failed to get rollback function: {}", e))?;
        
        // Call the rollback function
        let result = rollback.call(&mut store, ())
            .map_err(|e| anyhow!("Failed to call rollback function: {}", e))?;
        
        if result < 0 {
            return Err(anyhow!("Rollback failed with code {}", result));
        }
        
        // Get the CDC messages that were pushed
        let cdc_messages = self.cdc_messages.clone();
        
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
            Err(anyhow!("No CDC messages found for block {}", height))
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
    
    /// Push a CDC message
    ///
    /// This is called by the host function implementation
    ///
    /// # Arguments
    ///
    /// * `message` - The CDC message to push
    pub fn push_cdc_message(&mut self, message: CdcMessage) {
        self.cdc_messages.push(message);
    }
    
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

#[cfg(all(test, feature = "host"))]
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
        
        // Test inverting an Update message
        let update_message = CdcMessage {
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
                operation: CdcOperation::Update,
                table: "test_table".to_string(),
                key: "test_key".to_string(),
                before: Some(serde_json::json!({
                    "field1": "old_value",
                    "field2": 21
                })),
                after: Some(serde_json::json!({
                    "field1": "new_value",
                    "field2": 42
                })),
            },
        };
        
        let inverse = runtime.invert_cdc_message(&update_message, 122).unwrap();
        
        assert_eq!(inverse.payload.operation, CdcOperation::Update);
        assert_eq!(inverse.payload.table, "test_table");
        assert_eq!(inverse.payload.key, "test_key");
        assert_eq!(inverse.payload.before, update_message.payload.after);
        assert_eq!(inverse.payload.after, update_message.payload.before);
        
        // Test inverting a Delete message
        let delete_message = CdcMessage {
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
                operation: CdcOperation::Delete,
                table: "test_table".to_string(),
                key: "test_key".to_string(),
                before: Some(serde_json::json!({
                    "field1": "value1",
                    "field2": 42
                })),
                after: None,
            },
        };
        
        let inverse = runtime.invert_cdc_message(&delete_message, 122).unwrap();
        
        assert_eq!(inverse.payload.operation, CdcOperation::Create);
        assert_eq!(inverse.payload.table, "test_table");
        assert_eq!(inverse.payload.key, "test_key");
        assert_eq!(inverse.payload.before, None);
        assert_eq!(inverse.payload.after, delete_message.payload.before);
    }
}

// Stub implementation for when the host feature is not enabled
#[cfg(not(feature = "host"))]
pub struct WasmRuntime;

#[cfg(not(feature = "host"))]
impl WasmRuntime {
    pub fn new<P: AsRef<std::path::Path>>(_wasm_path: P) -> crate::error::Result<Self> {
        Ok(Self)
    }
    
    pub fn from_bytes(_wasm_bytes: &[u8]) -> crate::error::Result<Self> {
        Ok(Self)
    }
}