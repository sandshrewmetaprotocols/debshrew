//! WASM utilities for debshrew-runtime
//!
//! This module provides utilities for working with WebAssembly modules
//! in the debshrew runtime.

use std::alloc::{alloc, Layout};

/// Allocate memory in the WASM module and return a pointer to it
///
/// This function allocates memory in the WASM module and returns a pointer to it.
/// The memory is not freed automatically and must be freed by the caller.
///
/// # Arguments
///
/// * `size` - The size of the memory to allocate
///
/// # Returns
///
/// A pointer to the allocated memory
pub fn alloc_memory(size: usize) -> *mut u8 {
    unsafe {
        let layout = Layout::array::<u8>(size).unwrap();
        alloc(layout)
    }
}

/// Convert a Rust slice to an array buffer layout
///
/// This function converts a Rust slice to an array buffer layout
/// that can be passed to the host environment.
///
/// # Arguments
///
/// * `data` - The data to convert
///
/// # Returns
///
/// A vector containing the array buffer layout
pub fn to_arraybuffer_layout(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(4 + data.len());
    result.extend_from_slice(&(data.len() as u32).to_le_bytes());
    result.extend_from_slice(data);
    result
}

/// Convert a pointer to a Vec<u8>
///
/// This function converts a pointer to a Vec<u8> by reading the length
/// prefix and then copying the data.
///
/// # Arguments
///
/// * `ptr` - The pointer to convert
///
/// # Returns
///
/// A vector containing the data
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
///
/// This function copies a Vec<u8> to a pointer.
///
/// # Arguments
///
/// * `data` - The data to copy
/// * `ptr` - The pointer to copy to
pub fn vec_to_ptr(data: &[u8], ptr: i32) {
    unsafe {
        std::ptr::copy_nonoverlapping(data.as_ptr(), ptr as *mut u8, data.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arraybuffer_layout() {
        let data = b"Hello, world!";
        let layout = to_arraybuffer_layout(data);
        
        // Check length prefix
        assert_eq!(layout.len(), data.len() + 4);
        assert_eq!(u32::from_le_bytes([layout[0], layout[1], layout[2], layout[3]]), data.len() as u32);
        
        // Check data
        assert_eq!(&layout[4..], data);
    }
}