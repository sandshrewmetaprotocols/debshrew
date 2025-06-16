//! WASM export functions for debshrew-runtime
//!
//! This module provides utility functions for exporting data from WASM modules.

use crate::wasm;

/// Convert a byte slice to an ArrayBuffer layout (length prefix + data)
///
/// # Arguments
///
/// * `v` - The byte slice
pub fn to_arraybuffer_layout<T: AsRef<[u8]>>(v: T) -> Vec<u8> {
    wasm::to_arraybuffer_layout(v.as_ref())
}

/// Export bytes to the host environment
///
/// This function leaks memory, but that's okay because the host will
/// read the data and then drop the WASM instance.
///
/// # Arguments
///
/// * `v` - The bytes to export
///
/// # Returns
///
/// A pointer to the exported bytes
pub fn export_bytes(v: Vec<u8>) -> i32 {
    let response: Vec<u8> = to_arraybuffer_layout(&v);
    Box::leak(Box::new(response)).as_mut_ptr() as usize as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arraybuffer_layout() {
        let data = b"Test data";
        let layout = to_arraybuffer_layout(data);
        
        // Check length prefix
        assert_eq!(layout.len(), data.len() + 4);
        assert_eq!(u32::from_le_bytes([layout[0], layout[1], layout[2], layout[3]]), data.len() as u32);
        
        // Check data
        assert_eq!(&layout[4..], data);
    }
}