//! Standard I/O utilities for debshrew-runtime
//!
//! This module provides utilities for standard input/output operations
//! in the debshrew WASM runtime.

use crate::imports::__stdout;
use std::fmt::{Error, Write};

/// A struct representing standard output
pub struct Stdout(());

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> Result<(), Error> {
        let bytes = s.as_bytes();
        let mut encoded = Vec::with_capacity(4 + bytes.len());
        encoded.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        encoded.extend_from_slice(bytes);
        
        unsafe {
            __stdout(encoded.as_ptr() as i32);
        }
        
        Ok(())
    }
}

/// Get a handle to standard output
pub fn stdout() -> Stdout {
    Stdout(())
}

/// Write to standard output
pub fn write_stdout(msg: &str) {
    let bytes = msg.as_bytes();
    let mut encoded = Vec::with_capacity(4 + bytes.len());
    encoded.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    encoded.extend_from_slice(bytes);
    
    unsafe {
        __stdout(encoded.as_ptr() as i32);
    }
}

/// Write to standard error
pub fn write_stderr(msg: &str) {
    let bytes = msg.as_bytes();
    let mut encoded = Vec::with_capacity(4 + bytes.len());
    encoded.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    encoded.extend_from_slice(bytes);
    
    unsafe {
        crate::imports::__stderr(encoded.as_ptr() as i32);
    }
}

/// Write a formatted string to standard output, with a newline
pub fn println(args: std::fmt::Arguments) {
    use std::fmt::Write;
    let mut stdout = stdout();
    writeln!(stdout, "{}", args).unwrap();
}

/// Write a formatted string to standard error, with a newline
pub fn eprintln(args: std::fmt::Arguments) {
    use std::fmt::Write;
    let mut stdout = stdout();
    writeln!(stdout, "{}", args).unwrap();
}

/// Write a formatted string to standard output
pub fn print(args: std::fmt::Arguments) {
    use std::fmt::Write;
    let mut stdout = stdout();
    write!(stdout, "{}", args).unwrap();
}

/// Write a formatted string to standard error
pub fn eprint(args: std::fmt::Arguments) {
    use std::fmt::Write;
    let mut stdout = stdout();
    write!(stdout, "{}", args).unwrap();
}

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {{
        $crate::stdio::println(format_args!($($arg)*));
    }};
}

#[macro_export]
macro_rules! eprintln {
    ($($arg:tt)*) => {{
        $crate::stdio::eprintln(format_args!($($arg)*));
    }};
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        $crate::stdio::print(format_args!($($arg)*));
    }};
}

#[macro_export]
macro_rules! eprint {
    ($($arg:tt)*) => {{
        $crate::stdio::eprint(format_args!($($arg)*));
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    #[cfg(feature = "test-utils")]
    fn test_stdout_write() {
        let mut stdout = stdout();
        write!(stdout, "Hello, world!").unwrap();
        // With test-utils, this should not panic
    }
}
