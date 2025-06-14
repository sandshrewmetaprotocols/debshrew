//! Debshrew Test Consumer
//!
//! This is a simple Kafka consumer for testing Debshrew CDC messages.
//! It consumes messages from a Kafka topic and logs them to stdout.

use clap::{Parser, Subcommand};
use debshrew_support::{CdcMessage, CdcOperation};
use env_logger::Env;
use log::{error, info, warn};
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::Message;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tokio::signal;
use tokio::time;

/// Kafka consumer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
struct KafkaConfig {
    /// Bootstrap servers
    bootstrap_servers: String,
    
    /// Topic
    topic: String,
    
    /// Consumer group ID
    group_id: String,
    
    /// Auto offset reset
    #[serde(default = "default_auto_offset_reset")]
    auto_offset_reset: String,
    
    /// Session timeout
    #[serde(default = "default_session_timeout")]
    session_timeout_ms: u64,
}

fn default_auto_offset_reset() -> String {
    "earliest".to_string()
}

fn default_session_timeout() -> u64 {
    30000
}

impl Default for KafkaConfig {
    fn default() -> Self {
        Self {
            bootstrap_servers: "localhost:9092".to_string(),
            topic: "cdc-events".to_string(),
            group_id: "debshrew-test-consumer".to_string(),
            auto_offset_reset: default_auto_offset_reset(),
            session_timeout_ms: default_session_timeout(),
        }
    }
}

/// Debshrew Test Consumer CLI
#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// Subcommand
    #[clap(subcommand)]
    command: Commands,
}

/// CLI commands
#[derive(Subcommand)]
enum Commands {
    /// Run the test consumer
    Run {
        /// Path to the sink configuration file
        #[clap(short, long)]
        sink_config: Option<PathBuf>,
        
        /// Kafka bootstrap servers
        #[clap(short, long)]
        bootstrap_servers: Option<String>,
        
        /// Kafka topic
        #[clap(short, long)]
        topic: Option<String>,
        
        /// Consumer group ID
        #[clap(short, long)]
        group_id: Option<String>,
        
        /// Auto offset reset (earliest, latest)
        #[clap(short = 'o', long)]
        auto_offset_reset: Option<String>,
        
        /// Pretty print JSON
        #[clap(short, long)]
        pretty: bool,
        
        /// Log level
        #[clap(short, long, default_value = "info")]
        log_level: String,
    },
}

/// Main function
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse command line arguments
    let cli = Cli::parse();
    
    // Run the appropriate command
    match cli.command {
        Commands::Run {
            sink_config,
            bootstrap_servers,
            topic,
            group_id,
            auto_offset_reset,
            pretty,
            log_level,
        } => {
            // Initialize logger
            env_logger::Builder::from_env(Env::default().default_filter_or(&log_level)).init();
            
            // Load configuration
            let mut config = if let Some(config_path) = sink_config {
                info!("Loading configuration from {}", config_path.display());
                let config_str = std::fs::read_to_string(config_path)?;
                serde_json::from_str::<KafkaConfig>(&config_str)?
            } else {
                KafkaConfig::default()
            };
            
            // Override configuration with command line arguments
            if let Some(servers) = bootstrap_servers {
                config.bootstrap_servers = servers;
            }
            
            if let Some(t) = topic {
                config.topic = t;
            }
            
            if let Some(gid) = group_id {
                config.group_id = gid;
            }
            
            if let Some(offset) = auto_offset_reset {
                config.auto_offset_reset = offset;
            }
            
            // Create Kafka consumer
            info!("Connecting to Kafka at {}", config.bootstrap_servers);
            
            // Add debug logging for Kafka client configuration
            log::debug!("Kafka configuration:");
            log::debug!("  bootstrap.servers: {}", config.bootstrap_servers);
            log::debug!("  group.id: {}", config.group_id);
            log::debug!("  auto.offset.reset: {}", config.auto_offset_reset);
            log::debug!("  session.timeout.ms: {}", config.session_timeout_ms);
            
            // Create client config with debug logging
            let mut client_config = ClientConfig::new();
            client_config
                .set("group.id", &config.group_id)
                .set("bootstrap.servers", &config.bootstrap_servers)
                .set("enable.auto.commit", "true")
                .set("auto.offset.reset", &config.auto_offset_reset)
                .set("session.timeout.ms", &config.session_timeout_ms.to_string())
                .set("debug", "all")
                // Add client.rack to help with broker selection
                .set("client.rack", "local")
                // Add broker.address.family to prefer IPv4
                .set("broker.address.family", "v4")
                // Add client.id for better identification
                .set("client.id", "debshrew-test-consumer");
            
            // Create consumer
            let consumer: StreamConsumer = client_config.create()?;
            
            // Subscribe to the topic
            info!("Subscribing to topic {}", config.topic);
            consumer.subscribe(&[&config.topic])?;
            
            // Handle Ctrl+C
            let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel();
            
            tokio::spawn(async move {
                signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
                info!("Received Ctrl+C, shutting down...");
                let _ = shutdown_tx.send(());
            });
            
            // Main consumer loop
            info!("Starting consumer loop");
            
            let mut message_count = 0;
            let start_time = std::time::Instant::now();
            
            tokio::select! {
                _ = async {
                    loop {
                        match consumer.recv().await {
                            Ok(msg) => {
                                message_count += 1;
                                
                                // Get the payload
                                if let Some(payload) = msg.payload() {
                                    // Parse the payload as a CDC message
                                    match serde_json::from_slice::<CdcMessage>(payload) {
                                        Ok(cdc_message) => {
                                            // Log the message
                                            let operation_str = match cdc_message.payload.operation {
                                                CdcOperation::Create => "CREATE",
                                                CdcOperation::Update => "UPDATE",
                                                CdcOperation::Delete => "DELETE",
                                            };
                                            
                                            info!("Received CDC message: {} {} {} (block: {})",
                                                operation_str,
                                                cdc_message.payload.table,
                                                cdc_message.payload.key,
                                                cdc_message.header.block_height);
                                            
                                            // Print the full message if requested
                                            if pretty {
                                                match serde_json::to_string_pretty(&cdc_message) {
                                                    Ok(json) => println!("{}", json),
                                                    Err(e) => warn!("Failed to serialize CDC message: {}", e),
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            warn!("Failed to parse CDC message: {}", e);
                                            if let Ok(text) = std::str::from_utf8(payload) {
                                                warn!("Raw payload: {}", text);
                                            }
                                        }
                                    }
                                }
                                
                                // Log progress every 100 messages
                                if message_count % 100 == 0 {
                                    let elapsed = start_time.elapsed();
                                    let rate = message_count as f64 / elapsed.as_secs_f64();
                                    info!("Processed {} messages ({:.2} msgs/sec)", message_count, rate);
                                }
                            }
                            Err(e) => {
                                error!("Error while receiving message: {}", e);
                                time::sleep(Duration::from_secs(1)).await;
                            }
                        }
                    }
                } => {}
                _ = &mut shutdown_rx => {
                    info!("Shutting down consumer");
                }
            }
            
            info!("Consumer stopped");
            info!("Processed {} messages in total", message_count);
        }
    }
    
    Ok(())
}
