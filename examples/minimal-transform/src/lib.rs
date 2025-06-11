// Minimal transform that doesn't use wasm-bindgen
// This only uses the core WebAssembly features

// Import the necessary functions from the host
#[link(wasm_import_module = "env")]
extern "C" {
    fn __stdout(ptr: i32);
    fn __height() -> i32;
    fn __block_hash() -> i32;
    fn __load(ptr: i32);
}

// Static state for our transform
static mut TRANSFORM_STATE: Option<MinimalTransform> = None;

#[derive(Clone)]
struct MinimalTransform {
    // No state needed for this minimal example
}

impl Default for MinimalTransform {
    fn default() -> Self {
        Self {}
    }
}

// Helper function to write to stdout
fn write_stdout(msg: &str) {
    let bytes = msg.as_bytes();
    let mut encoded = Vec::with_capacity(4 + bytes.len());
    encoded.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    encoded.extend_from_slice(bytes);
    unsafe {
        __stdout(encoded.as_ptr() as i32);
    }
}

// Helper function to get the current block height
fn get_height() -> u32 {
    unsafe { __height() as u32 }
}

// Helper function to get the current block hash
fn get_block_hash() -> Vec<u8> {
    unsafe {
        let length = __block_hash();
        if length <= 0 {
            return Vec::new();
        }
        
        let mut buffer = vec![0u8; length as usize];
        __load(buffer.as_mut_ptr() as i32);
        buffer
    }
}

// Simple hex encoding function
fn to_hex(bytes: &[u8]) -> String {
    let mut hex = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        hex.push_str(&format!("{:02x}", byte));
    }
    hex
}

// Implementation of our transform
impl MinimalTransform {
    fn process_block(&mut self) -> i32 {
        let height = get_height();
        let hash = get_block_hash();
        
        write_stdout(&format!("Processing block {} with hash {}\n", height, to_hex(&hash)));
        
        // We're not creating any CDC messages in this minimal example
        write_stdout("Block processing complete\n");
        
        0 // Success
    }
    
    fn rollback(&mut self) -> i32 {
        let height = get_height();
        write_stdout(&format!("Rolling back to height {}\n", height));
        
        // We're not creating any CDC messages in this minimal example
        write_stdout("Rollback complete\n");
        
        0 // Success
    }
}

// Export the process_block function
#[no_mangle]
pub extern "C" fn process_block() -> i32 {
    unsafe {
        // Initialize the transform if it doesn't exist
        if TRANSFORM_STATE.is_none() {
            TRANSFORM_STATE = Some(MinimalTransform::default());
        }
        
        // Process the block
        if let Some(transform) = TRANSFORM_STATE.as_mut() {
            transform.process_block()
        } else {
            -1 // Error
        }
    }
}

// Export the rollback function
#[no_mangle]
pub extern "C" fn rollback() -> i32 {
    unsafe {
        // Initialize the transform if it doesn't exist
        if TRANSFORM_STATE.is_none() {
            TRANSFORM_STATE = Some(MinimalTransform::default());
        }
        
        // Process the rollback
        if let Some(transform) = TRANSFORM_STATE.as_mut() {
            transform.rollback()
        } else {
            -1 // Error
        }
    }
}