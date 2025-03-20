//! Utility functions for debshrew
//!
//! This module provides utility functions used throughout the debshrew project.

use crate::error::Result;
use chrono::{DateTime, TimeZone, Utc};
use lazy_static::lazy_static;
use regex::Regex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Get the current timestamp in milliseconds since the Unix epoch
///
/// # Returns
///
/// The current timestamp in milliseconds
///
/// # Examples
///
/// ```
/// use debshrew_support::utils::now_ms;
///
/// let timestamp = now_ms();
/// assert!(timestamp > 0);
/// ```
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis() as u64
}

/// Get the current timestamp as a UTC DateTime
///
/// # Returns
///
/// The current timestamp as a UTC DateTime
///
/// # Examples
///
/// ```
/// use debshrew_support::utils::now_utc;
///
/// let now = now_utc();
/// ```
pub fn now_utc() -> DateTime<Utc> {
    Utc::now()
}

/// Convert a Unix timestamp in milliseconds to a UTC DateTime
///
/// # Arguments
///
/// * `timestamp_ms` - The Unix timestamp in milliseconds
///
/// # Returns
///
/// The UTC DateTime
///
/// # Examples
///
/// ```
/// use debshrew_support::utils::timestamp_ms_to_datetime;
/// use chrono::Datelike;
///
/// let dt = timestamp_ms_to_datetime(1609459200000); // 2021-01-01T00:00:00Z
/// assert_eq!(dt.year(), 2021);
/// assert_eq!(dt.month(), 1);
/// assert_eq!(dt.day(), 1);
/// ```
pub fn timestamp_ms_to_datetime(timestamp_ms: u64) -> DateTime<Utc> {
    let secs = (timestamp_ms / 1000) as i64;
    let nsecs = ((timestamp_ms % 1000) * 1_000_000) as u32;
    Utc.timestamp_opt(secs, nsecs).unwrap()
}

/// Convert a UTC DateTime to a Unix timestamp in milliseconds
///
/// # Arguments
///
/// * `dt` - The UTC DateTime
///
/// # Returns
///
/// The Unix timestamp in milliseconds
///
/// # Examples
///
/// ```
/// use chrono::{TimeZone, Utc};
/// use debshrew_support::utils::datetime_to_timestamp_ms;
///
/// let dt = Utc.with_ymd_and_hms(2021, 1, 1, 0, 0, 0).unwrap();
/// let ts = datetime_to_timestamp_ms(&dt);
/// assert_eq!(ts, 1609459200000);
/// ```
pub fn datetime_to_timestamp_ms(dt: &DateTime<Utc>) -> u64 {
    (dt.timestamp() as u64 * 1000) + (dt.timestamp_subsec_millis() as u64)
}

/// Validate a Bitcoin block hash
///
/// # Arguments
///
/// * `hash` - The block hash to validate
///
/// # Returns
///
/// `true` if the hash is valid, `false` otherwise
///
/// # Examples
///
/// ```
/// use debshrew_support::utils::is_valid_block_hash;
///
/// assert!(is_valid_block_hash("000000000000000000024bead8df69990852c202db0e0097c1a12ea637d7e96d"));
/// assert!(!is_valid_block_hash("invalid"));
/// ```
pub fn is_valid_block_hash(hash: &str) -> bool {
    lazy_static! {
        static ref BLOCK_HASH_REGEX: Regex = Regex::new(r"^[0-9a-f]{64}$").unwrap();
    }
    BLOCK_HASH_REGEX.is_match(hash)
}

/// Validate a Bitcoin transaction ID
///
/// # Arguments
///
/// * `txid` - The transaction ID to validate
///
/// # Returns
///
/// `true` if the transaction ID is valid, `false` otherwise
///
/// # Examples
///
/// ```
/// use debshrew_support::utils::is_valid_txid;
///
/// assert!(is_valid_txid("f4184fc596403b9d638783cf57adfe4c75c605f6356fbc91338530e9831e9e16"));
/// assert!(!is_valid_txid("invalid"));
/// ```
pub fn is_valid_txid(txid: &str) -> bool {
    lazy_static! {
        static ref TXID_REGEX: Regex = Regex::new(r"^[0-9a-f]{64}$").unwrap();
    }
    TXID_REGEX.is_match(txid)
}

/// Generate a unique ID for a CDC message
///
/// # Arguments
///
/// * `source` - The source of the CDC message
/// * `table` - The table name
/// * `key` - The record key
/// * `block_height` - The block height
///
/// # Returns
///
/// A unique ID for the CDC message
///
/// # Examples
///
/// ```
/// use debshrew_support::utils::generate_cdc_message_id;
///
/// let id = generate_cdc_message_id("test_source", "test_table", "test_key", 123456);
/// ```
pub fn generate_cdc_message_id(source: &str, table: &str, key: &str, block_height: u32) -> String {
    format!("{}:{}:{}:{}", source, table, key, block_height)
}

/// Truncate a string to a maximum length with ellipsis
///
/// # Arguments
///
/// * `s` - The string to truncate
/// * `max_len` - The maximum length
///
/// # Returns
///
/// The truncated string
///
/// # Examples
///
/// ```
/// use debshrew_support::utils::truncate_string;
///
/// assert_eq!(truncate_string("Hello, world!", 5), "Hello...");
/// assert_eq!(truncate_string("Hello", 10), "Hello");
/// ```
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

/// Parse a URL and validate it
///
/// # Arguments
///
/// * `url_str` - The URL string to parse
///
/// # Returns
///
/// The parsed URL
///
/// # Errors
///
/// Returns an error if the URL is invalid
///
/// # Examples
///
/// ```
/// use debshrew_support::utils::parse_url;
///
/// let url = parse_url("http://example.com").unwrap();
/// assert_eq!(url.host_str(), Some("example.com"));
/// ```
pub fn parse_url(url_str: &str) -> Result<url::Url> {
    let url = url::Url::parse(url_str)?;
    
    // Validate URL scheme
    if url.scheme() != "http" && url.scheme() != "https" {
        return Err(format!("Invalid URL scheme: {}", url.scheme()).into());
    }
    
    // Validate URL host
    if url.host_str().is_none() {
        return Err("URL has no host".into());
    }
    
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;
    
    #[test]
    fn test_now_ms() {
        let t1 = now_ms();
        sleep(Duration::from_millis(10));
        let t2 = now_ms();
        assert!(t2 > t1);
    }
    
    #[test]
    fn test_timestamp_conversions() {
        let now = now_utc();
        let ts = datetime_to_timestamp_ms(&now);
        let dt = timestamp_ms_to_datetime(ts);
        
        // Allow for small differences due to precision loss
        let diff = (now.timestamp_millis() - dt.timestamp_millis()).abs();
        assert!(diff < 10);
    }
    
    #[test]
    fn test_block_hash_validation() {
        assert!(is_valid_block_hash("000000000000000000024bead8df69990852c202db0e0097c1a12ea637d7e96d"));
        assert!(is_valid_block_hash("0000000000000000000000000000000000000000000000000000000000000000"));
        assert!(!is_valid_block_hash("invalid"));
        assert!(!is_valid_block_hash("000000000000000000024BEAD8DF69990852C202DB0E0097C1A12EA637D7E96D")); // uppercase
        assert!(!is_valid_block_hash("000000000000000000024bead8df69990852c202db0e0097c1a12ea637d7e96")); // too short
    }
    
    #[test]
    fn test_txid_validation() {
        assert!(is_valid_txid("f4184fc596403b9d638783cf57adfe4c75c605f6356fbc91338530e9831e9e16"));
        assert!(!is_valid_txid("invalid"));
        assert!(!is_valid_txid("F4184FC596403B9D638783CF57ADFE4C75C605F6356FBC91338530E9831E9E16")); // uppercase
    }
    
    #[test]
    fn test_generate_cdc_message_id() {
        let id = generate_cdc_message_id("test_source", "test_table", "test_key", 123456);
        assert_eq!(id, "test_source:test_table:test_key:123456");
    }
    
    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("Hello, world!", 5), "Hello...");
        assert_eq!(truncate_string("Hello", 10), "Hello");
        assert_eq!(truncate_string("", 5), "");
    }
    
    #[test]
    fn test_parse_url() {
        let url = parse_url("http://example.com").unwrap();
        assert_eq!(url.scheme(), "http");
        assert_eq!(url.host_str(), Some("example.com"));
        
        let url = parse_url("https://example.com:8080/path?query=value").unwrap();
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host_str(), Some("example.com"));
        assert_eq!(url.port(), Some(8080));
        assert_eq!(url.path(), "/path");
        
        assert!(parse_url("ftp://example.com").is_err());
        assert!(parse_url("invalid").is_err());
    }
}