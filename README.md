# debshrew

A framework for building deterministic CDC streams from Bitcoin metaprotocol state.

Debshrew enables building highly available, reorg-aware ETL pipelines that transform metaprotocol state into standardized CDC streams consumable by the debezium ecosystem.

[![Crates.io](https://img.shields.io/crates/v/debshrew.svg)](https://crates.io/crates/debshrew)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

## Core Features

- WASM-based transformation programs
- Deterministic CDC generation from metashrew views
- Automatic reorg handling with state rollbacks
- 6-block caching for reorg protection
- Debezium-compatible CDC output
- Extensible metaprotocol support

## Architecture

Debshrew consists of the following components:

- **debshrew**: The main service that synchronizes with metashrew, processes blocks, and outputs CDC messages
- **debshrew-runtime**: The WASM runtime for executing transform modules
- **debshrew-support**: Common utilities and shared code
- **transform modules**: WASM modules that implement the transform logic for specific metaprotocols

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  Metashrew  │────▶│  Debshrew   │────▶│  CDC Sink   │
└─────────────┘     └─────────────┘     └─────────────┘
                          │
                          ▼
                    ┌─────────────┐
                    │   WASM      │
                    │  Transform  │
                    └─────────────┘
```

## Installation

### Prerequisites

- Rust 1.70 or later
- A running metashrew instance
- For WASM transform development: `wasm32-unknown-unknown` target (`rustup target add wasm32-unknown-unknown`)

### Building from Source

```bash
# Clone the repository
git clone https://github.com/example/debshrew.git
cd debshrew

# Build the project
cargo build --release
```

## Usage

### Running Debshrew

```bash
# Run with a configuration file
cargo run --release -- run --config config.json

# Run with command line arguments
cargo run --release -- run \
  --metashrew-url http://localhost:8080 \
  --transform path/to/transform.wasm \
  --sink-type kafka \
  --sink-config kafka-config.json \
  --cache-size 6 \
  --start-height 100000
```

### Configuration

Debshrew can be configured using a JSON configuration file:

```json
{
  "metashrew": {
    "url": "http://localhost:8080",
    "username": "user",
    "password": "password",
    "timeout": 30,
    "max_retries": 3,
    "retry_delay": 1000
  },
  "transform": {
    "path": "path/to/transform.wasm"
  },
  "sink": {
    "type": "kafka",
    "bootstrap_servers": "localhost:9092",
    "topic": "cdc-events",
    "client_id": "debshrew",
    "batch_size": 100,
    "flush_interval": 1000
  },
  "cache_size": 6,
  "start_height": 100000,
  "log_level": "info"
}
```

### Sink Types

Debshrew supports the following sink types:

- **Kafka**: Sends CDC messages to a Kafka topic
- **PostgreSQL**: Applies CDC messages to a PostgreSQL database
- **File**: Writes CDC messages to a file
- **Console**: Outputs CDC messages to the console

## Elasticsearch Integration

Debshrew now includes a complete Elasticsearch integration with Kibana GUI for visualizing CDC events. This setup provides:

- **Elasticsearch**: Search and analytics engine for storing CDC data
- **Kibana**: Web-based GUI for data visualization and exploration
- **Kafka Connect**: Automated pipeline from Kafka topics to Elasticsearch

### Quick Start with Elasticsearch

```bash
# Start the complete CDC to Elasticsearch pipeline
./run-cdc-elasticsearch-pipeline.sh start

# Setup the Elasticsearch connector
./run-cdc-elasticsearch-pipeline.sh setup

# Test the pipeline
./test-cdc-elasticsearch.sh

# Access Kibana GUI
open http://localhost:5601
```

### Service URLs

- **Kibana GUI**: http://localhost:5601
- **Elasticsearch API**: http://localhost:9200
- **Kafka Connect API**: http://localhost:8083
- **Kafka Bootstrap Server**: localhost:9092

For detailed setup instructions, see [ELASTICSEARCH_SETUP.md](ELASTICSEARCH_SETUP.md).

## Creating Transform Modules

Transform modules are WASM modules that implement the `DebTransform` trait. They are responsible for querying metashrew views, detecting changes, and generating CDC messages.

Here's a simple example:

```rust
use debshrew_runtime::*;

#[derive(Debug, Default, Clone)]
struct MyTransform {
    // State fields
}

impl DebTransform for MyTransform {
    fn process_block(&mut self) -> Result<()> {
        // Get current block info
        let height = get_height();
        let hash = get_block_hash();
        
        // Query metashrew views
        let result = view("my_view".to_string(), vec![])?;
        
        // Generate CDC messages
        let message = CdcMessage {
            header: CdcHeader {
                source: "my_transform".to_string(),
                timestamp: chrono::Utc::now(),
                block_height: height,
                block_hash: hex::encode(&hash),
                transaction_id: None,
            },
            payload: CdcPayload {
                operation: CdcOperation::Create,
                table: "my_table".to_string(),
                key: "my_key".to_string(),
                before: None,
                after: Some(serde_json::json!({
                    "field1": "value1",
                    "field2": 42
                })),
            },
        };
        
        // Push CDC message
        self.push_message(message)?;
        
        Ok(())
    }
}

// Declare the transform module
declare_transform!(MyTransform);
```

See the [examples](examples/) directory for more examples.

## CDC Message Format

Debshrew generates CDC messages in the following format:

```json
{
  "header": {
    "source": "my_transform",
    "timestamp": "2023-01-01T00:00:00Z",
    "block_height": 123456,
    "block_hash": "000000000000000000024bead8df69990852c202db0e0097c1a12ea637d7e96d",
    "transaction_id": "tx123"
  },
  "payload": {
    "operation": "create",
    "table": "my_table",
    "key": "my_key",
    "before": null,
    "after": {
      "field1": "value1",
      "field2": 42
    }
  }
}
```

## Documentation

- [Installation Guide](docs/installation.md)
- [Quick Start Guide](docs/quickstart.md)
- [Configuration Guide](docs/configuration.md)
- [Architecture Overview](docs/architecture.md)
- [CDC Concepts](docs/cdc-concepts.md)
- [Metashrew Integration](docs/metashrew-integration.md)
- [WASM Transform Guide](docs/wasm-transform-guide.md)
- [API Reference](docs/api/)

To build the documentation locally:

```bash
# Install mkdocs and required plugins
pip install mkdocs mkdocs-material mkdocs-minify-plugin mkdocs-git-revision-date-localized-plugin mkdocstrings

# Build the documentation
cd docs
mkdocs build

# Serve the documentation locally
mkdocs serve
```

Then open http://localhost:8000 in your browser.
