//! Serialization utilities for debshrew
//!
//! This module provides utilities for serializing and deserializing data
//! in various formats, including JSON, bincode, and hex.

use crate::error::{Error, Result};
use serde::{de::DeserializeOwned, Serialize};

/// Serialize a value to JSON
///
/// # Arguments
///
/// * `value` - The value to serialize
///
/// # Returns
///
/// The serialized value as a JSON string
///
/// # Errors
///
/// Returns an error if serialization fails
///
/// # Examples
///
/// ```
/// use debshrew_support::serialization::serialize_to_json;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Person {
///     name: String,
///     age: u32,
/// }
///
/// let person = Person {
///     name: "Alice".to_string(),
///     age: 30,
/// };
///
/// let json = serialize_to_json(&person).unwrap();
/// assert_eq!(json, r#"{"name":"Alice","age":30}"#);
/// ```
pub fn serialize_to_json<T: Serialize>(value: &T) -> Result<String> {
    serde_json::to_string(value).map_err(Error::from)
}

/// Serialize a value to a JSON value
///
/// # Arguments
///
/// * `value` - The value to serialize
///
/// # Returns
///
/// The serialized value as a JSON value
///
/// # Errors
///
/// Returns an error if serialization fails
pub fn serialize_to_json_value<T: Serialize>(value: &T) -> Result<serde_json::Value> {
    serde_json::to_value(value).map_err(Error::from)
}

/// Deserialize a JSON string to a value
///
/// # Arguments
///
/// * `json` - The JSON string to deserialize
///
/// # Returns
///
/// The deserialized value
///
/// # Errors
///
/// Returns an error if deserialization fails
///
/// # Examples
///
/// ```
/// use debshrew_support::serialization::deserialize_from_json;
/// use serde::Deserialize;
///
/// #[derive(Deserialize, PartialEq, Debug)]
/// struct Person {
///     name: String,
///     age: u32,
/// }
///
/// let json = r#"{"name":"Alice","age":30}"#;
/// let person: Person = deserialize_from_json(json).unwrap();
/// assert_eq!(person.name, "Alice");
/// assert_eq!(person.age, 30);
/// ```
pub fn deserialize_from_json<T: DeserializeOwned>(json: &str) -> Result<T> {
    serde_json::from_str(json).map_err(Error::from)
}

/// Serialize a value to bincode
///
/// # Arguments
///
/// * `value` - The value to serialize
///
/// # Returns
///
/// The serialized value as a byte vector
///
/// # Errors
///
/// Returns an error if serialization fails
pub fn serialize<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    bincode::serialize(value).map_err(Error::from)
}

/// Deserialize a bincode byte vector to a value
///
/// # Arguments
///
/// * `data` - The byte vector to deserialize
///
/// # Returns
///
/// The deserialized value
///
/// # Errors
///
/// Returns an error if deserialization fails
pub fn deserialize<T: DeserializeOwned>(data: &[u8]) -> Result<T> {
    bincode::deserialize(data).map_err(Error::from)
}

/// Encode bytes as a hex string
///
/// # Arguments
///
/// * `bytes` - The bytes to encode
///
/// # Returns
///
/// The encoded hex string
pub fn encode_hex(bytes: &[u8]) -> String {
    hex::encode(bytes)
}

/// Decode a hex string to bytes
///
/// # Arguments
///
/// * `hex_str` - The hex string to decode
///
/// # Returns
///
/// The decoded bytes
///
/// # Errors
///
/// Returns an error if decoding fails
pub fn decode_hex(hex_str: &str) -> Result<Vec<u8>> {
    hex::decode(hex_str).map_err(Error::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct TestStruct {
        name: String,
        value: u32,
    }

    #[test]
    fn test_json_serialization() {
        let test = TestStruct {
            name: "test".to_string(),
            value: 42,
        };

        let json = serialize_to_json(&test).unwrap();
        assert_eq!(json, r#"{"name":"test","value":42}"#);

        let deserialized: TestStruct = deserialize_from_json(&json).unwrap();
        assert_eq!(deserialized, test);
    }

    #[test]
    fn test_bincode_serialization() {
        let test = TestStruct {
            name: "test".to_string(),
            value: 42,
        };

        let encoded = serialize(&test).unwrap();
        let deserialized: TestStruct = deserialize(&encoded).unwrap();
        assert_eq!(deserialized, test);
    }

    #[test]
    fn test_hex_encoding() {
        let bytes = vec![0x01, 0x02, 0x03, 0x04];
        let hex = encode_hex(&bytes);
        assert_eq!(hex, "01020304");

        let decoded = decode_hex(&hex).unwrap();
        assert_eq!(decoded, bytes);
    }
}
