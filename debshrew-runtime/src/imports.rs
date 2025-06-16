//! WASM import functions for debshrew-runtime
//!
//! This module defines the WASM import functions that are called by the host environment.

#[cfg(not(feature = "test-utils"))]
#[link(wasm_import_module = "env")]
extern "C" {
    pub fn __load(output: i32);
    pub fn __view(view_name: i32, input: i32) -> i32;
    pub fn __stdout(s: i32);
    pub fn __stderr(s: i32);
    pub fn __height() -> i32;
    pub fn __block_hash() -> i32;
    pub fn __get_state(key: i32) -> i32;
    pub fn __set_state(key: i32, value: i32) -> i32;
    pub fn __delete_state(key: i32) -> i32;
}

#[cfg(feature = "test-utils")]
pub mod exports {
    // Test implementation
    
    pub fn __load(_output: i32) {
        // Test implementation
    }
    
    pub fn __view(_view_name: i32, _input: i32) -> i32 {
        // Test implementation
        0
    }
    
    pub fn __stdout(_s: i32) {
        // Test implementation
    }
    
    pub fn __stderr(_s: i32) {
        // Test implementation
    }
    
    pub fn __height() -> i32 {
        // Test implementation
        0
    }
    
    pub fn __block_hash() -> i32 {
        // Test implementation
        0
    }
    
    pub fn __get_state(_key: i32) -> i32 {
        // Test implementation
        0
    }
    
    pub fn __set_state(_key: i32, _value: i32) -> i32 {
        // Test implementation
        0
    }
    
    pub fn __delete_state(_key: i32) -> i32 {
        // Test implementation
        0
    }
}

#[cfg(feature = "test-utils")]
pub use exports::*;

/// Convert a pointer to a Vec<u8>
pub fn ptr_to_vec(ptr: i32) -> Vec<u8> {
    unsafe {
        // First read the length (4 bytes)
        let p = ptr as *const u8;
        let len = u32::from_le_bytes([*p, *p.offset(1), *p.offset(2), *p.offset(3)]) as usize;

        // Then read the actual data
        let mut result = Vec::with_capacity(len);
        std::ptr::copy_nonoverlapping(p.offset(4), result.as_mut_ptr(), len);
        result.set_len(len);
        result
    }
}

/// Copy a Vec<u8> to a pointer
pub fn vec_to_ptr(data: &[u8], ptr: i32) {
    unsafe {
        std::ptr::copy_nonoverlapping(data.as_ptr(), ptr as *mut u8, data.len());
    }
}