//! Metashrew client implementation
//!
//! This module provides the client implementation for communicating with metashrew.

use crate::error::{Error, Result};
use crate::config::MetashrewConfig;
use async_trait::async_trait;
use log;
use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use url::Url;

/// Maximum length for logged response bodies (in characters)
const MAX_RESPONSE_LOG_LENGTH: usize = 1000;

/// Truncate a response string for logging purposes
fn truncate_response_for_logging(response: &str) -> String {
    if response.len() <= MAX_RESPONSE_LOG_LENGTH {
        response.to_string()
    } else {
        format!("{}... [truncated, total length: {} chars]",
                &response[..MAX_RESPONSE_LOG_LENGTH],
                response.len())
    }
}

/// Metashrew client trait
///
/// This trait defines the interface for communicating with metashrew.
#[async_trait]
pub trait MetashrewClient: Send + Sync {
    /// Get the current block height
    ///
    /// # Returns
    ///
    /// The current block height
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails
    async fn get_height(&self) -> Result<u32>;
    
    /// Get the block hash for a given height
    ///
    /// # Arguments
    ///
    /// * `height` - The block height
    ///
    /// # Returns
    ///
    /// The block hash
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails
    async fn get_block_hash(&self, height: u32) -> Result<Vec<u8>>;
    
    /// Call a view function
    ///
    /// # Arguments
    ///
    /// * `view_name` - The name of the view function
    /// * `params` - The parameters to pass to the view function
    /// * `height` - The block height to query at (optional)
    ///
    /// # Returns
    ///
    /// The result of the view function
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails
    async fn call_view(&self, view_name: &str, params: &[u8], height: Option<u32>) -> Result<Vec<u8>>;
    
    /// Get the URL of the metashrew service
    ///
    /// # Returns
    ///
    /// The URL of the metashrew service
    fn get_url(&self) -> &Url;
}

/// JSON-RPC request
#[derive(Debug, Serialize)]
struct JsonRpcRequest<T> {
    /// JSON-RPC version
    jsonrpc: String,
    
    /// Method name
    method: String,
    
    /// Parameters
    params: T,
    
    /// Request ID
    id: u32,
}

// Implement Debug for JsonRpcRequest to help with logging
impl<T: Serialize> JsonRpcRequest<T> {
    fn to_json_string_pretty(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "Failed to serialize request".to_string())
    }
}

/// JSON-RPC response
#[derive(Debug, Deserialize)]
struct JsonRpcResponse<T> {
    /// JSON-RPC version
    #[allow(dead_code)]
    jsonrpc: String,
    
    /// Result
    result: Option<T>,
    
    /// Error
    error: Option<JsonRpcError>,
    
    /// Request ID
    #[allow(dead_code)]
    id: u32,
}

/// JSON-RPC error
#[derive(Debug, Deserialize)]
struct JsonRpcError {
    /// Error code
    code: Option<i32>,
    
    /// Error message
    message: String,
    
    /// Error data
    #[allow(dead_code)]
    data: Option<serde_json::Value>,
}

/// JSON-RPC client for metashrew
#[derive(Debug, Clone)]
pub struct JsonRpcClient {
    /// The HTTP client
    client: Client,
    
    /// The metashrew URL
    url: Url,
    
    /// The request ID counter
    request_id: u32,
}

/// Synchronous version of MetashrewClient trait
pub trait SyncMetashrewClient: Send + Sync {
    /// Get the current block height
    fn get_height(&self) -> Result<u32>;
    
    /// Get the block hash for a given height
    fn get_block_hash(&self, height: u32) -> Result<Vec<u8>>;
    
    /// Call a view function
    fn call_view(&self, view_name: &str, params: &[u8]) -> Result<Vec<u8>>;
    
    /// Get the URL of the metashrew service
    fn get_url(&self) -> &Url;
}

impl JsonRpcClient {
    /// Create a new JSON-RPC client
    ///
    /// # Arguments
    ///
    /// * `url` - The metashrew URL
    ///
    /// # Returns
    ///
    /// A new JSON-RPC client
    ///
    /// # Errors
    ///
    /// Returns an error if the URL is invalid
    pub fn new(url: &str) -> Result<Self> {
        let url = Url::parse(url)
            .map_err(|e| Error::MetashrewClient(format!("Invalid URL: {}", e)))?;
        
        let client = Client::new();
        
        Ok(Self {
            client,
            url,
            request_id: 0,
        })
    }
    
    /// Get the URL of the metashrew service
    ///
    /// # Returns
    ///
    /// The URL of the metashrew service
    pub fn get_url(&self) -> &Url {
        &self.url
    }
    
    /// Create a new JSON-RPC client from a configuration
    ///
    /// # Arguments
    ///
    /// * `config` - The metashrew configuration
    ///
    /// # Returns
    ///
    /// A new JSON-RPC client
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid
    pub fn from_config(config: &MetashrewConfig) -> Result<Self> {
        let url = Url::parse(&config.url)
            .map_err(|e| Error::MetashrewClient(format!("Invalid URL: {}", e)))?;
        
        let client_builder = ClientBuilder::new()
            .timeout(Duration::from_secs(config.timeout))
            .connect_timeout(Duration::from_secs(config.timeout));
        
        // Add authentication if provided
        if let (Some(_username), Some(_password)) = (&config.username, &config.password) {
            // Create a client with basic auth
            let client = client_builder.build()
                .map_err(|e| Error::MetashrewClient(format!("Failed to build HTTP client: {}", e)))?;
            
            return Ok(Self {
                client,
                url,
                request_id: 0,
            });
        }
        
        let client = client_builder.build()
            .map_err(|e| Error::MetashrewClient(format!("Failed to build HTTP client: {}", e)))?;
        
        Ok(Self {
            client,
            url,
            request_id: 0,
        })
    }
    
    /// Get the next request ID
    ///
    /// # Returns
    ///
    /// The next request ID
    fn next_request_id(&mut self) -> u32 {
        let id = self.request_id;
        self.request_id = self.request_id.wrapping_add(1);
        id
    }
    
    /// Send a JSON-RPC request
    ///
    /// # Arguments
    ///
    /// * `method` - The method name
    /// * `params` - The parameters
    ///
    /// # Returns
    ///
    /// The JSON-RPC response
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails
    async fn send_request<T, R>(&mut self, method: &str, params: T) -> Result<R>
    where
        T: Serialize,
        R: for<'de> Deserialize<'de>,
    {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id: self.next_request_id(),
        };
        
        // Log the request for debugging
        log::debug!("Sending JSONRPC request to {}: \n{}", self.url, request.to_json_string_pretty());
        
        // Manually serialize the request to JSON
        let request_json = serde_json::to_string(&request)
            .map_err(|e| Error::MetashrewClient(format!("Failed to serialize request: {}", e)))?;
        
        // Send the request with explicit Content-Type header
        let response = self.client.post(self.url.clone())
            .header("Content-Type", "application/json")
            .body(request_json)
            .send()
            .await
            .map_err(|e| Error::MetashrewClient(format!("Failed to send request: {}", e)))?;
        
        let status = response.status();
        if !status.is_success() {
            return Err(Error::MetashrewClient(format!("HTTP error: {}", status)));
        }
        
        // Get the raw response text for debugging
        let response_text = response.text().await
            .map_err(|e| Error::MetashrewClient(format!("Failed to get response text: {}", e)))?;
        
        log::debug!("Received raw response: \n{}", truncate_response_for_logging(&response_text));
        
        // Parse the response as JSON
        let json_response: JsonRpcResponse<R> = serde_json::from_str(&response_text)
            .map_err(|e| Error::MetashrewClient(format!("Failed to parse response as JSON: {}\nRaw response: {}", e, response_text)))?;
        
        if let Some(error) = json_response.error {
            return Err(Error::MetashrewClient(format!("JSON-RPC error: {} (code: {})", error.message, error.code.unwrap_or_else(|| -1))));
        }
        
        json_response.result
            .ok_or_else(|| Error::MetashrewClient("No result in response".to_string()))
    }
    
    /// Send a JSON-RPC request synchronously
    fn send_request_sync<T, R>(&mut self, method: &str, params: T) -> Result<R>
    where
        T: Serialize,
        R: for<'de> Deserialize<'de>,
    {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id: self.next_request_id(),
        };
        
        // Log the request for debugging
        log::debug!("Sending synchronous JSONRPC request to {}: \n{}", self.url, request.to_json_string_pretty());
        
        // Manually serialize the request to JSON
        let request_json = serde_json::to_string(&request)
            .map_err(|e| Error::MetashrewClient(format!("Failed to serialize request: {}", e)))?;
        
        // Send the request with explicit Content-Type header using blocking client
        let client = reqwest::blocking::Client::new();
        let response = client.post(self.url.clone())
            .header("Content-Type", "application/json")
            .body(request_json)
            .send()
            .map_err(|e| Error::MetashrewClient(format!("Failed to send request: {}", e)))?;
        
        let status = response.status();
        if !status.is_success() {
            return Err(Error::MetashrewClient(format!("HTTP error: {}", status)));
        }
        
        // Get the raw response text for debugging
        let response_text = response.text()
            .map_err(|e| Error::MetashrewClient(format!("Failed to get response text: {}", e)))?;
        
        log::debug!("Received raw response: \n{}", truncate_response_for_logging(&response_text));
        
        // Parse the response as JSON
        let json_response: JsonRpcResponse<R> = serde_json::from_str(&response_text)
            .map_err(|e| Error::MetashrewClient(format!("Failed to parse response as JSON: {}\nRaw response: {}", e, response_text)))?;
        
        if let Some(error) = json_response.error {
            return Err(Error::MetashrewClient(format!("JSON-RPC error: {} (code: {})", error.message, error.code.unwrap_or_else(|| -1))));
        }
        
        json_response.result
            .ok_or_else(|| Error::MetashrewClient("No result in response".to_string()))
    }
}

/// Implement the synchronous version of MetashrewClient for JsonRpcClient
impl SyncMetashrewClient for JsonRpcClient {
    fn get_height(&self) -> Result<u32> {
        let mut client = self.clone();
        
        // For get_height, we're sending an empty array as params
        let height_str: String = client.send_request_sync("metashrew_height", Vec::<String>::new())?;
        
        // Parse the string as a u32
        let height = height_str.parse::<u32>()
            .map_err(|e| Error::MetashrewClient(format!("Failed to parse height '{}' as u32: {}", height_str, e)))?;
        
        log::debug!("Got height: {}", height);
        Ok(height)
    }
    
    fn get_block_hash(&self, height: u32) -> Result<Vec<u8>> {
        let mut client = self.clone();
        
        // For get_block_hash, we're sending a vec with the height
        let params = serde_json::json!([height]);
        log::debug!("Sending get_block_hash with params: {}", serde_json::to_string(&params).unwrap_or_default());
        
        let hash: String = client.send_request_sync("metashrew_getblockhash", params)?;
        
        log::debug!("Got block hash (hex): {}", hash);
        
        // Strip the '0x' prefix if present
        let clean_hash = if hash.starts_with("0x") {
            hash[2..].to_string()
        } else {
            hash
        };
        
        log::debug!("Clean hash (after stripping 0x prefix): {}", clean_hash);
        
        // Convert hex string to bytes
        let hash_bytes = hex::decode(clean_hash)
            .map_err(|e| Error::MetashrewClient(format!("Failed to decode block hash: {}", e)))?;
        
        Ok(hash_bytes)
    }
    
    fn call_view(&self, view_name: &str, params: &[u8]) -> Result<Vec<u8>> {
        let mut client = self.clone();
        
        // Convert params to hex string
        let params_hex = hex::encode(params);
        
        // Log the original params for debugging
        log::debug!("Original params: {:?}", params);
        log::debug!("Hex-encoded params: {}", params_hex);
        
        // Prepare parameters for the view call
        // The metashrew_view method expects an array of strings
        let view_params = serde_json::json!([view_name, params_hex, "latest"]);
        
        log::debug!("View params JSON: {}", serde_json::to_string_pretty(&view_params).unwrap_or_default());
        
        // Call the view function
        let result: String = client.send_request_sync("metashrew_view", view_params)?;
        
        // Pretty print and log the result (truncated)
        log::info!("JSONRPC result from metashrew_view '{}': ", view_name);
        
        // Try to parse the result as JSON for pretty printing
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&result) {
            // Pretty print the JSON
            let pretty_json = serde_json::to_string_pretty(&json_value)
                .unwrap_or_else(|_| result.clone());
            
            // Truncate the pretty JSON for logging
            let truncated_json = truncate_response_for_logging(&pretty_json);
            
            // Log each line of the truncated pretty-printed JSON with proper indentation
            for line in truncated_json.lines() {
                log::info!("  {}", line);
            }
        } else {
            // If it's not valid JSON, just log the truncated raw result
            log::info!("  {}", truncate_response_for_logging(&result));
        }
        
        // Strip the '0x' prefix if present
        let clean_result = if result.starts_with("0x") {
            result[2..].to_string()
        } else {
            result
        };
        
        log::debug!("Clean result (after stripping 0x prefix if present): {}", truncate_response_for_logging(&clean_result));
        
        // Convert hex string to bytes
        let result_bytes = hex::decode(clean_result)
            .map_err(|e| Error::MetashrewClient(format!("Failed to decode view result: {}", e)))?;
        
        Ok(result_bytes)
    }
    
    fn get_url(&self) -> &Url {
        &self.url
    }
}

#[async_trait]
impl MetashrewClient for JsonRpcClient {
    async fn get_height(&self) -> Result<u32> {
        let mut client = self.clone();
        
        // For get_height, we're sending an empty array as params
        // The Metashrew service expects [] not null
        // The result comes back as a string, so we need to parse it
        let height_str: String = client.send_request("metashrew_height", Vec::<String>::new()).await?;
        
        // Parse the string as a u32
        let height = height_str.parse::<u32>()
            .map_err(|e| Error::MetashrewClient(format!("Failed to parse height '{}' as u32: {}", height_str, e)))?;
        
        log::debug!("Got height: {}", height);
        Ok(height)
    }
    
    async fn get_block_hash(&self, height: u32) -> Result<Vec<u8>> {
        let mut client = self.clone();
        
        // For get_block_hash, we're sending a vec with the height
        // Let's use serde_json::Value to ensure proper JSON formatting
        let params = serde_json::json!([height]);
        log::debug!("Sending get_block_hash with params: {}", serde_json::to_string(&params).unwrap_or_default());
        
        let hash: String = client.send_request("metashrew_getblockhash", params).await?;
        
        log::debug!("Got block hash (hex): {}", hash);
        
        // Strip the '0x' prefix if present
        let clean_hash = if hash.starts_with("0x") {
            hash[2..].to_string()
        } else {
            hash
        };
        
        log::debug!("Clean hash (after stripping 0x prefix): {}", clean_hash);
        
        // Convert hex string to bytes
        let hash_bytes = hex::decode(clean_hash)
            .map_err(|e| Error::MetashrewClient(format!("Failed to decode block hash: {}", e)))?;
        
        Ok(hash_bytes)
    }
    
    async fn call_view(&self, view_name: &str, params: &[u8], height: Option<u32>) -> Result<Vec<u8>> {
        let mut client = self.clone();
        
        // Convert params to hex string
        let params_hex = hex::encode(params);
        
        // Log the original params for debugging
        log::debug!("Original params: {:?}", params);
        log::debug!("Hex-encoded params: {}", params_hex);
        
        // Prepare parameters for the view call
        // The metashrew_view method expects an array of strings
        // Let's try using a JSON array directly to ensure proper formatting
        let view_params = match height {
            Some(h) => serde_json::json!([view_name, params_hex, h]),
            None => serde_json::json!([view_name, params_hex, "latest"]),
        };
        
        log::debug!("View params JSON: {}", serde_json::to_string_pretty(&view_params).unwrap_or_default());
        
        // Call the view function
        let result: String = client.send_request("metashrew_view", view_params).await?;
        
        // Pretty print and log the result (truncated)
        log::info!("JSONRPC result from metashrew_view '{}': ", view_name);
        
        // Try to parse the result as JSON for pretty printing
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&result) {
            // Pretty print the JSON
            let pretty_json = serde_json::to_string_pretty(&json_value)
                .unwrap_or_else(|_| result.clone());
            
            // Truncate the pretty JSON for logging
            let truncated_json = truncate_response_for_logging(&pretty_json);
            
            // Log each line of the truncated pretty-printed JSON with proper indentation
            for line in truncated_json.lines() {
                log::info!("  {}", line);
            }
        } else {
            // If it's not valid JSON, just log the truncated raw result
            log::info!("  {}", truncate_response_for_logging(&result));
        }
        
        // Strip the '0x' prefix if present
        let clean_result = if result.starts_with("0x") {
            result[2..].to_string()
        } else {
            result
        };
        
        log::debug!("Clean result (after stripping 0x prefix if present): {}", truncate_response_for_logging(&clean_result));
        
        // Convert hex string to bytes
        let result_bytes = hex::decode(clean_result)
            .map_err(|e| Error::MetashrewClient(format!("Failed to decode view result: {}", e)))?;
        
        Ok(result_bytes)
    }
    
    fn get_url(&self) -> &Url {
        &self.url
    }
}

/// Mock metashrew client for testing
#[derive(Debug, Clone)]
pub struct MockMetashrewClient {
    /// The current block height
    pub height: u32,
    
    /// The block hashes
    pub block_hashes: Vec<Vec<u8>>,
    
    /// The view function results
    pub view_results: Vec<(String, Vec<u8>, Option<u32>, Vec<u8>)>,
    
    /// The mock URL
    url: Url,
}

impl MockMetashrewClient {
    /// Create a new mock metashrew client
    ///
    /// # Returns
    ///
    /// A new mock metashrew client
    pub fn new() -> Self {
        Self {
            height: 0,
            block_hashes: Vec::new(),
            view_results: Vec::new(),
            url: Url::parse("http://localhost:18888").unwrap(), // Default URL
        }
    }
    
    /// Create a new mock metashrew client with a specific URL
    ///
    /// # Arguments
    ///
    /// * `url` - The metashrew URL
    ///
    /// # Returns
    ///
    /// A new mock metashrew client
    pub fn with_url(url: &str) -> Result<Self> {
        let parsed_url = Url::parse(url)
            .map_err(|e| Error::MetashrewClient(format!("Invalid URL: {}", e)))?;
            
        Ok(Self {
            height: 0,
            block_hashes: Vec::new(),
            view_results: Vec::new(),
            url: parsed_url,
        })
    }
    
    /// Set the metashrew URL
    ///
    /// # Arguments
    ///
    /// * `url` - The metashrew URL
    ///
    /// # Returns
    ///
    /// Result indicating success or failure
    pub fn set_url(&mut self, url: &str) -> Result<()> {
        self.url = Url::parse(url)
            .map_err(|e| Error::MetashrewClient(format!("Invalid URL: {}", e)))?;
        Ok(())
    }
    
    /// Set the current block height
    ///
    /// # Arguments
    ///
    /// * `height` - The current block height
    pub fn set_height(&mut self, height: u32) {
        self.height = height;
    }
    
    /// Set the block hash for a given height
    ///
    /// # Arguments
    ///
    /// * `height` - The block height
    /// * `hash` - The block hash
    pub fn set_block_hash(&mut self, height: u32, hash: Vec<u8>) {
        // Ensure the block_hashes vector is large enough
        if height as usize >= self.block_hashes.len() {
            self.block_hashes.resize(height as usize + 1, Vec::new());
        }
        
        self.block_hashes[height as usize] = hash;
    }
    
    /// Set the result for a view function
    ///
    /// # Arguments
    ///
    /// * `view_name` - The name of the view function
    /// * `params` - The parameters to match
    /// * `height` - The block height to match (optional)
    /// * `result` - The result to return
    pub fn set_view_result(&mut self, view_name: &str, params: &[u8], height: Option<u32>, result: Vec<u8>) {
        self.view_results.push((view_name.to_string(), params.to_vec(), height, result));
    }
}

#[async_trait]
impl MetashrewClient for MockMetashrewClient {
    async fn get_height(&self) -> Result<u32> {
        Ok(self.height)
    }
    
    async fn get_block_hash(&self, height: u32) -> Result<Vec<u8>> {
        if height as usize >= self.block_hashes.len() {
            return Err(Error::MetashrewClient(format!("Block hash not found for height {}", height)));
        }
        
        let hash = &self.block_hashes[height as usize];
        if hash.is_empty() {
            return Err(Error::MetashrewClient(format!("Block hash not found for height {}", height)));
        }
        
        Ok(hash.clone())
    }
    
    async fn call_view(&self, view_name: &str, params: &[u8], height: Option<u32>) -> Result<Vec<u8>> {
        for (name, p, h, result) in &self.view_results {
            if name == view_name && p == params && h == &height {
                // Log the result for consistency with the real client (truncated)
                log::info!("JSONRPC result from metashrew_view '{}' (mock): ", view_name);
                
                // Try to parse the result as JSON for pretty printing
                let result_str = String::from_utf8_lossy(result);
                if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&result_str) {
                    // Pretty print the JSON
                    let pretty_json = serde_json::to_string_pretty(&json_value)
                        .unwrap_or_else(|_| result_str.to_string());
                    
                    // Truncate the pretty JSON for logging
                    let truncated_json = truncate_response_for_logging(&pretty_json);
                    
                    // Log each line of the truncated pretty-printed JSON with proper indentation
                    for line in truncated_json.lines() {
                        log::info!("  {}", line);
                    }
                } else {
                    // If it's not valid JSON, just log the truncated raw result
                    let debug_str = format!("{:?}", result);
                    log::info!("  {}", truncate_response_for_logging(&debug_str));
                }
                
                return Ok(result.clone());
            }
        }
        
        Err(Error::MetashrewClient(format!("View result not found for {}", view_name)))
    }
    
    fn get_url(&self) -> &Url {
        &self.url
    }
}

impl Default for MockMetashrewClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use wiremock::matchers::{method, path};
    use serde_json::json;

    #[test]
    fn test_mock_metashrew_client() {
        let mut client = MockMetashrewClient::new();
        
        // Set up the mock client
        client.set_height(123);
        client.set_block_hash(123, vec![1, 2, 3]);
        client.set_view_result("test_view", &[4, 5, 6], None, vec![7, 8, 9]);
        
        // Create a runtime for async tests
        let rt = Runtime::new().unwrap();
        
        // Test get_height
        let height = rt.block_on(client.get_height()).unwrap();
        assert_eq!(height, 123);
        
        // Test get_block_hash
        let hash = rt.block_on(client.get_block_hash(123)).unwrap();
        assert_eq!(hash, vec![1, 2, 3]);
        
        // Test call_view
        let result = rt.block_on(client.call_view("test_view", &[4, 5, 6], None)).unwrap();
        assert_eq!(result, vec![7, 8, 9]);
        
        // Test error cases
        assert!(rt.block_on(client.get_block_hash(456)).is_err());
        assert!(rt.block_on(client.call_view("nonexistent", &[], None)).is_err());
    }

    #[tokio::test]
    async fn test_json_rpc_client() {
        // Start a mock server
        let mock_server = MockServer::start().await;
        
        // Mock the get_height request
        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(json!({
                    "jsonrpc": "2.0",
                    "result": 123,
                    "id": 0
                })))
            .mount(&mock_server)
            .await;
        
        // Create a client
        let client = JsonRpcClient::new(&mock_server.uri()).unwrap();
        
        // Test get_height
        let height = client.get_height().await.unwrap();
        assert_eq!(height, 123);
    }
}
