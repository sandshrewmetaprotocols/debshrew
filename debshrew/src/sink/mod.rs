//! CDC sink implementations
//!
//! This module provides the CDC sink interfaces and implementations for
//! outputting CDC messages to various destinations.

use crate::config::SinkConfig;
use crate::error::{Error, Result};
use async_trait::async_trait;
use debshrew_support::CdcMessage;
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord, Producer};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Mutex as TokioMutex;
use postgres::types::ToSql;

/// CDC sink trait
///
/// This trait defines the interface for CDC sinks, which are responsible for
/// outputting CDC messages to various destinations.
#[async_trait]
pub trait CdcSink: Send + Sync {
    /// Send CDC messages to the sink
    ///
    /// # Arguments
    ///
    /// * `messages` - The CDC messages to send
    ///
    /// # Returns
    ///
    /// Ok(()) if the messages were sent successfully
    ///
    /// # Errors
    ///
    /// Returns an error if the messages cannot be sent
    async fn send(&self, messages: Vec<CdcMessage>) -> Result<()>;
    
    /// Flush the sink
    ///
    /// # Returns
    ///
    /// Ok(()) if the sink was flushed successfully
    ///
    /// # Errors
    ///
    /// Returns an error if the sink cannot be flushed
    async fn flush(&self) -> Result<()>;
    
    /// Close the sink
    ///
    /// # Returns
    ///
    /// Ok(()) if the sink was closed successfully
    ///
    /// # Errors
    ///
    /// Returns an error if the sink cannot be closed
    async fn close(&self) -> Result<()>;
}

/// Create a CDC sink from a configuration
///
/// # Arguments
///
/// * `config` - The sink configuration
///
/// # Returns
///
/// A new CDC sink
///
/// # Errors
///
/// Returns an error if the sink cannot be created
pub fn create_sink(config: &SinkConfig) -> Result<Box<dyn CdcSink>> {
    match config {
        SinkConfig::Kafka { bootstrap_servers, topic, client_id, batch_size, flush_interval } => {
            let sink = KafkaSink::new(
                bootstrap_servers,
                topic,
                client_id.as_deref(),
                *batch_size,
                *flush_interval,
            )?;
            Ok(Box::new(sink))
        }
        SinkConfig::Postgres { connection_string, schema, batch_size, flush_interval } => {
            let sink = PostgresSink::new(
                connection_string,
                schema,
                *batch_size,
                *flush_interval,
            )?;
            Ok(Box::new(sink))
        }
        SinkConfig::File { path, append, flush_interval } => {
            let sink = FileSink::new(path, *append, *flush_interval)?;
            Ok(Box::new(sink))
        }
        SinkConfig::Console { pretty_print } => {
            let sink = ConsoleSink::new(*pretty_print);
            Ok(Box::new(sink))
        }
    }
}

/// Kafka CDC sink
///
/// This sink sends CDC messages to a Kafka topic.
pub struct KafkaSink {
    /// The Kafka producer
    producer: FutureProducer,
    
    /// The Kafka topic
    topic: String,
    
    /// The batch size
    batch_size: usize,
    
    /// The flush interval in milliseconds
    flush_interval: u64,
}

impl KafkaSink {
    /// Create a new Kafka sink
    ///
    /// # Arguments
    ///
    /// * `bootstrap_servers` - The Kafka bootstrap servers
    /// * `topic` - The Kafka topic
    /// * `client_id` - The Kafka client ID (optional)
    /// * `batch_size` - The batch size
    /// * `flush_interval` - The flush interval in milliseconds
    ///
    /// # Returns
    ///
    /// A new Kafka sink
    ///
    /// # Errors
    ///
    /// Returns an error if the Kafka producer cannot be created
    pub fn new(
        bootstrap_servers: &str,
        topic: &str,
        client_id: Option<&str>,
        batch_size: usize,
        flush_interval: u64,
    ) -> Result<Self> {
        let mut client_config = ClientConfig::new();
        client_config.set("bootstrap.servers", bootstrap_servers);
        
        if let Some(id) = client_id {
            client_config.set("client.id", id);
        }
        
        // Set additional Kafka configuration
        client_config.set("message.timeout.ms", "5000");
        client_config.set("socket.keepalive.enable", "true");
        
        let producer: FutureProducer = client_config.create()
            .map_err(|e| Error::Kafka(format!("Failed to create Kafka producer: {}", e)))?;
        
        Ok(Self {
            producer,
            topic: topic.to_string(),
            batch_size,
            flush_interval,
        })
    }
}

#[async_trait]
impl CdcSink for KafkaSink {
    async fn send(&self, messages: Vec<CdcMessage>) -> Result<()> {
        // Process messages in batches
        for chunk in messages.chunks(self.batch_size) {
            // Process each message in the chunk
            for message in chunk {
                // Use the message key as the Kafka key
                let key = message.payload.key.clone();
                
                // Serialize the message to JSON
                let value = serde_json::to_string(message)
                    .map_err(|e| Error::Kafka(format!("Failed to serialize message: {}", e)))?;
                
                // Send the message to Kafka and wait for the result
                self.producer.send(
                    FutureRecord::to(&self.topic)
                        .key(&key)
                        .payload(&value),
                    Duration::from_millis(5000),
                )
                .await
                .map_err(|(e, _)| Error::Kafka(format!("Failed to send message: {}", e)))?;
            }
        }
        
        Ok(())
    }
    
    async fn flush(&self) -> Result<()> {
        self.producer.flush(Duration::from_millis(self.flush_interval))
            .map_err(|e| Error::Kafka(format!("Failed to flush Kafka producer: {}", e)))?;
        
        Ok(())
    }
    
    async fn close(&self) -> Result<()> {
        // Flush the producer before closing
        self.flush().await?;
        
        Ok(())
    }
}

/// PostgreSQL CDC sink
///
/// This sink sends CDC messages to a PostgreSQL database.
pub struct PostgresSink {
    /// The PostgreSQL connection string
    connection_string: String,
    
    /// The PostgreSQL schema
    schema: String,
    
    /// The batch size
    batch_size: usize,
    
    /// The flush interval in milliseconds
    flush_interval: u64,
    
    /// The message buffer
    buffer: Arc<TokioMutex<Vec<CdcMessage>>>,
}

impl PostgresSink {
    /// Create a new PostgreSQL sink
    ///
    /// # Arguments
    ///
    /// * `connection_string` - The PostgreSQL connection string
    /// * `schema` - The PostgreSQL schema
    /// * `batch_size` - The batch size
    /// * `flush_interval` - The flush interval in milliseconds
    ///
    /// # Returns
    ///
    /// A new PostgreSQL sink
    ///
    /// # Errors
    ///
    /// Returns an error if the PostgreSQL connection cannot be created
    pub fn new(
        connection_string: &str,
        schema: &str,
        batch_size: usize,
        flush_interval: u64,
    ) -> Result<Self> {
        // Validate the connection string by attempting to connect
        let _client = postgres::Client::connect(connection_string, postgres::NoTls)
            .map_err(|e| Error::Postgres(format!("Failed to connect to PostgreSQL: {}", e)))?;
        
        Ok(Self {
            connection_string: connection_string.to_string(),
            schema: schema.to_string(),
            batch_size,
            flush_interval,
            buffer: Arc::new(TokioMutex::new(Vec::new())),
        })
    }
    
    /// Apply CDC messages to a PostgreSQL database
    ///
    /// # Arguments
    ///
    /// * `messages` - The CDC messages to apply
    ///
    /// # Returns
    ///
    /// Ok(()) if the messages were applied successfully
    ///
    /// # Errors
    ///
    /// Returns an error if the messages cannot be applied
    async fn apply_messages(&self, messages: &[CdcMessage]) -> Result<()> {
        // Connect to PostgreSQL
        let mut client = postgres::Client::connect(&self.connection_string, postgres::NoTls)
            .map_err(|e| Error::Postgres(format!("Failed to connect to PostgreSQL: {}", e)))?;
        
        // Start a transaction
        client.batch_execute("BEGIN")
            .map_err(|e| Error::Postgres(format!("Failed to start transaction: {}", e)))?;
        
        // Process each message
        for message in messages {
            let table = format!("{}.{}", self.schema, message.payload.table);
            
            match message.payload.operation {
                debshrew_support::CdcOperation::Create => {
                    // Extract fields from the after state
                    if let Some(after) = &message.payload.after {
                        let fields: Vec<String> = after.as_object()
                            .ok_or_else(|| Error::Postgres("Invalid after state".to_string()))?
                            .keys()
                            .map(|k| k.to_string())
                            .collect();
                        
                        // Convert JSON values to strings for PostgreSQL
                        let values: Vec<String> = fields.iter()
                            .map(|f| after[f].to_string())
                            .collect();
                        
                        // Build the INSERT statement
                        let placeholders: Vec<String> = (1..=fields.len())
                            .map(|i| format!("${}", i))
                            .collect();
                        
                        let query = format!(
                            "INSERT INTO {} ({}) VALUES ({})",
                            table,
                            fields.join(", "),
                            placeholders.join(", ")
                        );
                        
                        // Execute the INSERT statement
                        let params: Vec<&(dyn ToSql + Sync)> = values.iter()
                            .map(|v| v as &(dyn ToSql + Sync))
                            .collect();
                        
                        client.execute(&query, &params)
                            .map_err(|e| Error::Postgres(format!("Failed to execute INSERT: {}", e)))?;
                    }
                }
                debshrew_support::CdcOperation::Update => {
                    // Extract fields from the after state
                    if let Some(after) = &message.payload.after {
                        let fields: Vec<String> = after.as_object()
                            .ok_or_else(|| Error::Postgres("Invalid after state".to_string()))?
                            .keys()
                            .map(|k| k.to_string())
                            .collect();
                        
                        // Convert JSON values to strings for PostgreSQL
                        let values: Vec<String> = fields.iter()
                            .map(|f| after[f].to_string())
                            .collect();
                        
                        // Build the UPDATE statement
                        let set_clauses: Vec<String> = fields.iter()
                            .enumerate()
                            .map(|(i, f)| format!("{} = ${}", f, i + 1))
                            .collect();
                        
                        let query = format!(
                            "UPDATE {} SET {} WHERE id = ${}",
                            table,
                            set_clauses.join(", "),
                            fields.len() + 1
                        );
                        
                        // Execute the UPDATE statement
                        let mut params: Vec<&(dyn ToSql + Sync)> = values.iter()
                            .map(|v| v as &(dyn ToSql + Sync))
                            .collect();
                        
                        let key = &message.payload.key;
                        params.push(key as &(dyn ToSql + Sync));
                        
                        client.execute(&query, &params)
                            .map_err(|e| Error::Postgres(format!("Failed to execute UPDATE: {}", e)))?;
                    }
                }
                debshrew_support::CdcOperation::Delete => {
                    // Build the DELETE statement
                    let query = format!("DELETE FROM {} WHERE id = $1", table);
                    
                    // Execute the DELETE statement
                    client.execute(&query, &[&message.payload.key])
                        .map_err(|e| Error::Postgres(format!("Failed to execute DELETE: {}", e)))?;
                }
            }
        }
        
        // Commit the transaction
        client.batch_execute("COMMIT")
            .map_err(|e| Error::Postgres(format!("Failed to commit transaction: {}", e)))?;
        
        Ok(())
    }
}

#[async_trait]
impl CdcSink for PostgresSink {
    async fn send(&self, messages: Vec<CdcMessage>) -> Result<()> {
        // Add messages to the buffer
        let mut buffer = self.buffer.lock().await;
        buffer.extend(messages);
        
        // Flush the buffer if it exceeds the batch size
        if buffer.len() >= self.batch_size {
            let messages_to_send = buffer.clone();
            buffer.clear();
            
            // Release the lock before applying messages
            drop(buffer);
            
            // Apply the messages
            self.apply_messages(&messages_to_send).await?;
        }
        
        Ok(())
    }
    
    async fn flush(&self) -> Result<()> {
        // Get the buffered messages
        let messages_to_send = {
            let mut buffer = self.buffer.lock().await;
            let messages = buffer.clone();
            buffer.clear();
            messages
        };
        
        // Apply the messages if there are any
        if !messages_to_send.is_empty() {
            // Use a timeout based on flush_interval
            let timeout = Duration::from_millis(self.flush_interval);
            let apply_future = self.apply_messages(&messages_to_send);
            
            match tokio::time::timeout(timeout, apply_future).await {
                Ok(result) => result?,
                Err(_) => return Err(Error::Postgres(format!("Flush operation timed out after {} ms", self.flush_interval))),
            }
        }
        
        Ok(())
    }
    
    async fn close(&self) -> Result<()> {
        // Flush the buffer before closing
        self.flush().await?;
        
        Ok(())
    }
}

/// File CDC sink
///
/// This sink writes CDC messages to a file.
pub struct FileSink {
    /// The file
    file: Arc<Mutex<File>>,
    
    /// The flush interval in milliseconds
    flush_interval: u64,
}

impl FileSink {
    /// Create a new file sink
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the file
    /// * `append` - Whether to append to the file
    /// * `flush_interval` - The flush interval in milliseconds
    ///
    /// # Returns
    ///
    /// A new file sink
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened
    pub fn new(path: &str, append: bool, flush_interval: u64) -> Result<Self> {
        // Create the parent directory if it doesn't exist
        if let Some(parent) = Path::new(path).parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| Error::File(format!("Failed to create directory: {}", e)))?;
            }
        }
        
        // Open the file
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(append)
            .truncate(!append)
            .open(path)
            .map_err(|e| Error::File(format!("Failed to open file: {}", e)))?;
        
        Ok(Self {
            file: Arc::new(Mutex::new(file)),
            flush_interval,
        })
    }
}

#[async_trait]
impl CdcSink for FileSink {
    async fn send(&self, messages: Vec<CdcMessage>) -> Result<()> {
        let mut file = self.file.lock()
            .map_err(|e| Error::File(format!("Failed to lock file: {}", e)))?;
        
        for message in messages {
            // Serialize the message to JSON
            let json = serde_json::to_string(&message)
                .map_err(|e| Error::File(format!("Failed to serialize message: {}", e)))?;
            
            // Write the message to the file
            writeln!(file, "{}", json)
                .map_err(|e| Error::File(format!("Failed to write to file: {}", e)))?;
        }
        
        Ok(())
    }
    
    async fn flush(&self) -> Result<()> {
        // Use a timeout based on flush_interval
        let timeout = Duration::from_millis(self.flush_interval);
        
        let flush_future = async {
            let mut file = self.file.lock()
                .map_err(|e| Error::File(format!("Failed to lock file: {}", e)))?;
            
            file.flush()
                .map_err(|e| Error::File(format!("Failed to flush file: {}", e)))?;
            
            Ok(())
        };
        
        match tokio::time::timeout(timeout, flush_future).await {
            Ok(result) => result,
            Err(_) => Err(Error::File(format!("Flush operation timed out after {} ms", self.flush_interval))),
        }
    }
    
    async fn close(&self) -> Result<()> {
        // Flush the file before closing
        self.flush().await?;
        
        Ok(())
    }
}

/// Console CDC sink
///
/// This sink writes CDC messages to the console.
pub struct ConsoleSink {
    /// Whether to pretty-print the messages
    pretty_print: bool,
}

impl ConsoleSink {
    /// Create a new console sink
    ///
    /// # Arguments
    ///
    /// * `pretty_print` - Whether to pretty-print the messages
    ///
    /// # Returns
    ///
    /// A new console sink
    pub fn new(pretty_print: bool) -> Self {
        Self { pretty_print }
    }
}

#[async_trait]
impl CdcSink for ConsoleSink {
    async fn send(&self, messages: Vec<CdcMessage>) -> Result<()> {
        for message in messages {
            // Serialize the message to JSON
            let json = if self.pretty_print {
                serde_json::to_string_pretty(&message)
                    .map_err(|e| Error::Generic(format!("Failed to serialize message: {}", e)))?
            } else {
                serde_json::to_string(&message)
                    .map_err(|e| Error::Generic(format!("Failed to serialize message: {}", e)))?
            };
            
            // Write the message to the console
            println!("{}", json);
        }
        
        Ok(())
    }
    
    async fn flush(&self) -> Result<()> {
        // No need to flush the console
        Ok(())
    }
    
    async fn close(&self) -> Result<()> {
        // No need to close the console
        Ok(())
    }
}

/// Null CDC sink
///
/// This sink discards all CDC messages.
pub struct NullSink;

impl NullSink {
    /// Create a new null sink
    ///
    /// # Returns
    ///
    /// A new null sink
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl CdcSink for NullSink {
    async fn send(&self, _messages: Vec<CdcMessage>) -> Result<()> {
        // Discard all messages
        Ok(())
    }
    
    async fn flush(&self) -> Result<()> {
        // No need to flush
        Ok(())
    }
    
    async fn close(&self) -> Result<()> {
        // No need to close
        Ok(())
    }
}

impl Default for NullSink {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use debshrew_support::{CdcHeader, CdcOperation, CdcPayload};
    use chrono::Utc;
    use std::io::Read;
    use tempfile::tempdir;
    use tokio::runtime::Runtime;

    fn create_test_message() -> CdcMessage {
        CdcMessage {
            header: CdcHeader {
                source: "test".to_string(),
                timestamp: Utc::now().timestamp_millis() as u64,
                block_height: 123,
                block_hash: "000000000000000000024bead8df69990852c202db0e0097c1a12ea637d7e96d".to_string(),
                transaction_id: None,
            },
            payload: CdcPayload {
                operation: CdcOperation::Create,
                table: "test_table".to_string(),
                key: "test_key".to_string(),
                before: None,
                after: Some(serde_json::json!({
                    "field1": "value1",
                    "field2": 42
                })),
            },
        }
    }

    #[test]
    fn test_null_sink() {
        let sink = NullSink::new();
        let rt = Runtime::new().unwrap();
        
        // Test send
        let result = rt.block_on(sink.send(vec![create_test_message()]));
        assert!(result.is_ok());
        
        // Test flush
        let result = rt.block_on(sink.flush());
        assert!(result.is_ok());
        
        // Test close
        let result = rt.block_on(sink.close());
        assert!(result.is_ok());
    }

    #[test]
    fn test_console_sink() {
        let sink = ConsoleSink::new(true);
        let rt = Runtime::new().unwrap();
        
        // Test send
        let result = rt.block_on(sink.send(vec![create_test_message()]));
        assert!(result.is_ok());
        
        // Test flush
        let result = rt.block_on(sink.flush());
        assert!(result.is_ok());
        
        // Test close
        let result = rt.block_on(sink.close());
        assert!(result.is_ok());
    }

    #[test]
    fn test_file_sink() {
        // Create a temporary directory
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.json");
        
        // Create a file sink
        let sink = FileSink::new(file_path.to_str().unwrap(), false, 1000).unwrap();
        let rt = Runtime::new().unwrap();
        
        // Test send
        let result = rt.block_on(sink.send(vec![create_test_message()]));
        assert!(result.is_ok());
        
        // Test flush
        let result = rt.block_on(sink.flush());
        assert!(result.is_ok());
        
        // Test close
        let result = rt.block_on(sink.close());
        assert!(result.is_ok());
        
        // Verify the file contents
        let mut file = File::open(file_path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        
        assert!(contents.contains("test_table"));
        assert!(contents.contains("test_key"));
        assert!(contents.contains("value1"));
        assert!(contents.contains("42"));
    }
}