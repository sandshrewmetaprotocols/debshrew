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
pub mod externs {
    use std::io::{self, Write};
    
    /// Write to stdout in test environment
    pub fn write_to_stdout(s: &str) {
        print!("{}", s);
        io::stdout().flush().unwrap();
    }
    
    /// Write to stderr in test environment
    pub fn write_to_stderr(s: &str) {
        eprint!("{}", s);
        io::stderr().flush().unwrap();
    }
}

#[cfg(feature = "test-utils")]
pub mod exports {
    use super::externs;
    use super::ptr_to_vec;
    use std::cell::RefCell;
    use std::collections::HashMap;
    
    // Thread-local storage for test state
    thread_local! {
        static TEST_STATE: RefCell<HashMap<Vec<u8>, Vec<u8>>> = RefCell::new(HashMap::new());
        static TEST_HEIGHT: RefCell<u32> = RefCell::new(0);
        static TEST_HASH: RefCell<Vec<u8>> = RefCell::new(Vec::new());
    }
    
    /// Set the test block height
    pub fn set_test_height(height: u32) {
        TEST_HEIGHT.with(|h| {
            *h.borrow_mut() = height;
        });
    }
    
    /// Set the test block hash
    pub fn set_test_hash(hash: Vec<u8>) {
        TEST_HASH.with(|h| {
            *h.borrow_mut() = hash;
        });
    }
    
    /// Clear the test state
    pub fn clear_test_state() {
        TEST_STATE.with(|s| {
            s.borrow_mut().clear();
        });
    }
    
    pub fn __load(_output: i32) {
        // In a real implementation, this would load data into the provided buffer
        // For testing, we do nothing
    }
    
    pub fn __view(_view_name: i32, _input: i32) -> i32 {
        // Test implementation
        0
    }
    
    pub fn __stdout(_s: i32) {
        // Safe implementation that doesn't use ptr_to_vec
        // Just print a placeholder message
        externs::write_to_stdout("[Test stdout output]");
    }
    
    pub fn __stderr(_s: i32) {
        // Safe implementation that doesn't use ptr_to_vec
        // Just print a placeholder message
        externs::write_to_stderr("[Test stderr output]");
    }
    
    pub fn __height() -> i32 {
        TEST_HEIGHT.with(|h| *h.borrow() as i32)
    }
    
    pub fn __block_hash() -> i32 {
        TEST_HASH.with(|h| h.borrow().len() as i32)
    }
    
    pub fn __get_state(key: i32) -> i32 {
        let key_data = ptr_to_vec(key);
        TEST_STATE.with(|s| {
            match s.borrow().get(&key_data) {
                Some(value) => value.len() as i32,
                None => 0,
            }
        })
    }
    
    pub fn __set_state(key: i32, value: i32) -> i32 {
        let key_data = ptr_to_vec(key);
        let value_data = ptr_to_vec(value);
        TEST_STATE.with(|s| {
            s.borrow_mut().insert(key_data, value_data);
            1
        })
    }
    
    pub fn __delete_state(key: i32) -> i32 {
        let key_data = ptr_to_vec(key);
        TEST_STATE.with(|s| {
            if s.borrow_mut().remove(&key_data).is_some() {
                1
            } else {
                0
            }
        })
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