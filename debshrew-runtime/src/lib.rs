//! Runtime library for debshrew
//!
//! This crate provides the runtime environment for debshrew transform modules,
//! including the WASM host interface, transform traits, and CDC message generation.

pub mod exports;
pub mod imports;
pub mod transform;
pub mod wasm;
pub mod error;

pub use crate::transform::{DebTransform, TransformResult};
pub use crate::error::{Error, Result};
pub use crate::wasm::WasmRuntime;
pub use anyhow;
pub use debshrew_support::{CdcMessage, CdcHeader, CdcOperation, CdcPayload, TransformState};
pub use serde::{Serialize, Deserialize};
pub use serde_json;

/// Safe wrapper for calling a view and loading its result
pub fn view(view_name: String, input: Vec<u8>) -> Result<Vec<u8>> {
    // Encode view name with length prefix
    let name_bytes = view_name.as_bytes();
    let mut encoded_name = Vec::with_capacity(4 + name_bytes.len());
    encoded_name.extend_from_slice(&(name_bytes.len() as u32).to_le_bytes());
    encoded_name.extend_from_slice(name_bytes);

    // Encode input with length prefix
    let mut encoded_input = Vec::with_capacity(4 + input.len());
    encoded_input.extend_from_slice(&(input.len() as u32).to_ne_bytes());
    encoded_input.extend_from_slice(&input);

    let length = unsafe { imports::__view(encoded_name.as_ptr() as i32, encoded_input.as_ptr() as i32) };
    if length <= 0 {
        return Err(anyhow::anyhow!("View call failed with length {}", length));
    }
    
    let mut buffer = vec![0u8; length as usize];
    unsafe { imports::__load(buffer.as_mut_ptr() as i32) };
    Ok(buffer)
}

/// Safe wrapper to get current block height
pub fn get_height() -> u32 {
    unsafe { imports::__height() as u32 }
}

/// Safe wrapper to get current block hash
pub fn get_block_hash() -> Vec<u8> {
    let length = unsafe { imports::__block_hash() };
    if length <= 0 {
        return Vec::new();
    }
    
    let mut buffer = vec![0u8; length as usize];
    unsafe { imports::__load(buffer.as_mut_ptr() as i32) };
    buffer
}

/// Get a value from the transform state
pub fn get_state(key: &[u8]) -> Option<Vec<u8>> {
    let encoded_key = exports::to_arraybuffer_layout(key);
    let length = unsafe { imports::__get_state(encoded_key.as_ptr() as i32) };
    if length <= 0 {
        return None;
    }
    
    let mut buffer = vec![0u8; length as usize];
    unsafe { imports::__load(buffer.as_mut_ptr() as i32) };
    Some(buffer)
}

/// Set a value in the transform state
pub fn set_state(key: &[u8], value: &[u8]) {
    let encoded_key = exports::to_arraybuffer_layout(key);
    let encoded_value = exports::to_arraybuffer_layout(value);
    unsafe { imports::__set_state(encoded_key.as_ptr() as i32, encoded_value.as_ptr() as i32) };
}

/// Delete a value from the transform state
pub fn delete_state(key: &[u8]) -> bool {
    let encoded_key = exports::to_arraybuffer_layout(key);
    unsafe { imports::__delete_state(encoded_key.as_ptr() as i32) > 0 }
}

/// Push a CDC message to the host
pub fn push_cdc_message(message: &CdcMessage) -> Result<()> {
    let serialized = serde_json::to_vec(message)
        .map_err(|e| anyhow::anyhow!("Failed to serialize CDC message: {}", e))?;
    
    let encoded = exports::to_arraybuffer_layout(&serialized);
    
    let result = unsafe { imports::__push_cdc_message(encoded.as_ptr() as i32) };
    if result < 0 {
        return Err(anyhow::anyhow!("Failed to push CDC message"));
    }
    
    Ok(())
}

/// Serialize parameters for a view function
pub fn serialize_params<T: Serialize>(params: &T) -> Result<Vec<u8>> {
    serde_json::to_vec(params)
        .map_err(|e| anyhow::anyhow!("Failed to serialize parameters: {}", e))
}

/// Deserialize the result from a view function
pub fn deserialize_result<T: for<'de> Deserialize<'de>>(result: &[u8]) -> Result<T> {
    serde_json::from_slice(result)
        .map_err(|e| anyhow::anyhow!("Failed to deserialize result: {}", e))
}

/// Safely write to stdout
pub fn write_stdout(msg: &str) {
    let bytes = msg.as_bytes();
    let mut encoded = Vec::with_capacity(4 + bytes.len());
    encoded.extend_from_slice(&(bytes.len() as u32).to_ne_bytes());
    encoded.extend_from_slice(bytes);
    unsafe {
        imports::__stdout(encoded.as_ptr() as i32);
    }
}

/// Safely write to stderr
pub fn write_stderr(msg: &str) {
    let bytes = msg.as_bytes();
    let mut encoded = Vec::with_capacity(4 + bytes.len());
    encoded.extend_from_slice(&(bytes.len() as u32).to_ne_bytes());
    encoded.extend_from_slice(bytes);
    unsafe {
        imports::__stderr(encoded.as_ptr() as i32);
    }
}

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {{
        $crate::write_stdout(&(format!($($arg)*) + "\n"));
    }};
}

#[macro_export]
macro_rules! eprintln {
    ($($arg:tt)*) => {{
        $crate::write_stderr(&(format!($($arg)*) + "\n"));
    }};
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        $crate::write_stdout(&format!($($arg)*));
    }};
}

#[macro_export]
macro_rules! eprint {
    ($($arg:tt)*) => {{
        $crate::write_stderr(&format!($($arg)*));
    }};
}

/// Declare a transform module
///
/// This macro generates the necessary WASM exports for a transform module.
#[macro_export]
macro_rules! declare_transform {
    ($transform:ty) => {
        use debshrew_runtime::push_cdc_message;
        use debshrew_runtime::Result;
        
        static mut INSTANCE: Option<$transform> = None;

        impl $transform {
            pub fn save(&self) -> Result<()> {
                unsafe {
                    INSTANCE = Some(self.clone());
                    Ok(())
                }
            }

            pub fn load() -> Result<Self> {
                unsafe {
                    if let Some(instance) = &INSTANCE {
                        return Ok(instance.clone());
                    }
                    
                    let instance = Self::default();
                    INSTANCE = Some(instance.clone());
                    Ok(instance)
                }
            }
            
            // Helper method to push CDC messages
            pub fn push_message(&self, message: debshrew_runtime::CdcMessage) -> Result<()> {
                push_cdc_message(&message)
            }
        }

        #[no_mangle]
        pub fn process_block() -> i32 {
            unsafe {
                // Load instance
                let mut instance = match <$transform>::load() {
                    Ok(instance) => instance,
                    Err(e) => {
                        $crate::eprintln!("Failed to load transform state: {}", e);
                        return -1;
                    }
                };
                
                // Process block
                match instance.process_block() {
                    Ok(()) => {
                        // Save state after successful processing
                        if let Err(e) = instance.save() {
                            $crate::eprintln!("Failed to save transform state: {}", e);
                            return -1;
                        }
                        
                        // Success - CDC messages were pushed via push_message
                        0
                    }
                    Err(e) => {
                        $crate::eprintln!("Transform failed: {}", e);
                        -1
                    }
                }
            }
        }

        #[no_mangle]
        pub fn rollback() -> i32 {
            unsafe {
                // Load instance
                let mut instance = match <$transform>::load() {
                    Ok(instance) => instance,
                    Err(e) => {
                        $crate::eprintln!("Failed to load transform state: {}", e);
                        return -1;
                    }
                };
                
                // Process rollback
                match instance.rollback() {
                    Ok(()) => {
                        // Save state after successful rollback
                        if let Err(e) = instance.save() {
                            $crate::eprintln!("Failed to save transform state: {}", e);
                            return -1;
                        }
                        
                        // Success - CDC messages were pushed via push_message
                        0
                    }
                    Err(e) => {
                        $crate::eprintln!("Rollback failed: {}", e);
                        -1
                    }
                }
            }
        }
    };
}