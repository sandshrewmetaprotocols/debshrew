//! Debshrew CLI
//!
//! This is the main binary for the debshrew service, which provides a CLI
//! interface for running the service.

use clap::{Parser, Subcommand};
use debshrew::{
    client::JsonRpcClient,
    config::{Config, SinkConfig},
    create_sink,
    error::Result,
    BlockSynchronizer,
    WasmRuntime,
};
use env_logger::Env;
use log::{error, info};
use std::path::PathBuf;
use tokio::signal;

/// Debshrew CLI
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
    /// Run the debshrew service
    Run {
        /// Path to the configuration file
        #[clap(short, long)]
        config: Option<PathBuf>,
        
        /// Metashrew URL
        #[clap(short, long)]
        metashrew_url: Option<String>,
        
        /// Path to the transform WASM module
        #[clap(short, long)]
        transform: Option<PathBuf>,
        
        /// Sink type (kafka, postgres, file, console)
        #[clap(short, long)]
        sink_type: Option<String>,
        
        /// Path to the sink configuration file
        #[clap(short = 'f', long)]
        sink_config: Option<PathBuf>,
        
        /// Block cache size
        #[clap(short = 'z', long, default_value = "6")]
        cache_size: u32,
        
        /// Starting block height
        #[clap(short = 'b', long)]
        start_height: Option<u32>,
        
        /// Log level
        #[clap(short, long, default_value = "info")]
        log_level: String,
    },
}

/// Main function
#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let cli = Cli::parse();
    
    // Run the appropriate command
    match cli.command {
        Commands::Run {
            config,
            metashrew_url,
            transform,
            sink_type,
            sink_config,
            cache_size,
            start_height,
            log_level,
        } => {
            // Initialize logger
            env_logger::Builder::from_env(Env::default().default_filter_or(&log_level)).init();
            
            // Load configuration
            let config = if let Some(config_path) = config {
                info!("Loading configuration from {}", config_path.display());
                Config::from_file(config_path)?
            } else {
                // Create configuration from command line arguments
                let metashrew_url = metashrew_url.ok_or_else(|| {
                    error!("Metashrew URL is required when not using a configuration file");
                    "Metashrew URL is required when not using a configuration file"
                })?;
                
                let transform_path = transform.ok_or_else(|| {
                    error!("Transform path is required when not using a configuration file");
                    "Transform path is required when not using a configuration file"
                })?;
                
                let sink_config = if let Some(sink_type) = sink_type {
                    match sink_type.as_str() {
                        "kafka" => {
                            let sink_config_path = sink_config.ok_or_else(|| {
                                error!("Sink configuration is required for Kafka sink");
                                "Sink configuration is required for Kafka sink"
                            })?;
                            
                            let sink_config_str = std::fs::read_to_string(sink_config_path)?;
                            let kafka_config: serde_json::Value = serde_json::from_str(&sink_config_str)?;
                            
                            SinkConfig::Kafka {
                                bootstrap_servers: kafka_config["bootstrap_servers"].as_str().unwrap_or("localhost:9092").to_string(),
                                topic: kafka_config["topic"].as_str().unwrap_or("cdc-events").to_string(),
                                client_id: kafka_config["client_id"].as_str().map(|s| s.to_string()),
                                batch_size: kafka_config["batch_size"].as_u64().unwrap_or(100) as usize,
                                flush_interval: kafka_config["flush_interval"].as_u64().unwrap_or(1000),
                            }
                        }
                        "postgres" => {
                            let sink_config_path = sink_config.ok_or_else(|| {
                                error!("Sink configuration is required for PostgreSQL sink");
                                "Sink configuration is required for PostgreSQL sink"
                            })?;
                            
                            let sink_config_str = std::fs::read_to_string(sink_config_path)?;
                            let postgres_config: serde_json::Value = serde_json::from_str(&sink_config_str)?;
                            
                            SinkConfig::Postgres {
                                connection_string: postgres_config["connection_string"].as_str().unwrap_or("").to_string(),
                                schema: postgres_config["schema"].as_str().unwrap_or("public").to_string(),
                                batch_size: postgres_config["batch_size"].as_u64().unwrap_or(100) as usize,
                                flush_interval: postgres_config["flush_interval"].as_u64().unwrap_or(1000),
                            }
                        }
                        "file" => {
                            let sink_config_path = sink_config.ok_or_else(|| {
                                error!("Sink configuration is required for file sink");
                                "Sink configuration is required for file sink"
                            })?;
                            
                            let sink_config_str = std::fs::read_to_string(sink_config_path)?;
                            let file_config: serde_json::Value = serde_json::from_str(&sink_config_str)?;
                            
                            SinkConfig::File {
                                path: file_config["path"].as_str().unwrap_or("cdc-events.json").to_string(),
                                append: file_config["append"].as_bool().unwrap_or(true),
                                flush_interval: file_config["flush_interval"].as_u64().unwrap_or(1000),
                            }
                        }
                        "console" => {
                            let pretty_print = if let Some(sink_config_path) = sink_config {
                                let sink_config_str = std::fs::read_to_string(sink_config_path)?;
                                let console_config: serde_json::Value = serde_json::from_str(&sink_config_str)?;
                                console_config["pretty_print"].as_bool().unwrap_or(false)
                            } else {
                                false
                            };
                            
                            SinkConfig::Console { pretty_print }
                        }
                        _ => {
                            error!("Invalid sink type: {}", sink_type);
                            return Err(format!("Invalid sink type: {}", sink_type).into());
                        }
                    }
                } else {
                    // Default to console sink
                    SinkConfig::Console { pretty_print: false }
                };
                
                Config {
                    metashrew: debshrew::config::MetashrewConfig {
                        url: metashrew_url,
                        username: None,
                        password: None,
                        timeout: 30,
                        max_retries: 3,
                        retry_delay: 1000,
                    },
                    transform: debshrew::config::TransformConfig {
                        path: transform_path.to_string_lossy().to_string(),
                    },
                    sink: sink_config,
                    cache_size,
                    start_height,
                    log_level,
                }
            };
            
            // Validate configuration
            config.validate()?;
            
            // Create metashrew client
            info!("Connecting to metashrew at {}", config.metashrew.url);
            let client = JsonRpcClient::from_config(&config.metashrew)?;
            
            // Load transform module
            info!("Loading transform module from {}", config.transform.path);
            let runtime = WasmRuntime::new(&config.transform.path)?;
            
            // Create CDC sink
            info!("Creating CDC sink");
            let sink = create_sink(&config.sink)?;
            
            // Create block synchronizer
            info!("Creating block synchronizer with cache size {}", config.cache_size);
            let mut synchronizer = BlockSynchronizer::new(client, runtime, sink, config.cache_size)?;
            
            // Set starting height if provided
            if let Some(height) = config.start_height {
                info!("Setting starting height to {}", height);
                synchronizer.set_starting_height(height);
            }
            
            // Run the synchronizer
            info!("Starting block synchronization");
            
            // Handle Ctrl+C
            let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel();
            
            tokio::spawn(async move {
                signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
                info!("Received Ctrl+C, shutting down...");
                let _ = shutdown_tx.send(());
            });
            
            // Run the synchronizer until shutdown signal
            tokio::select! {
                result = synchronizer.run() => {
                    if let Err(e) = result {
                        error!("Synchronizer error: {}", e);
                        return Err(e);
                    }
                }
                _ = &mut shutdown_rx => {
                    info!("Shutting down synchronizer");
                    synchronizer.stop();
                }
            }
            
            info!("Debshrew service stopped");
        }
    }
    
    Ok(())
}