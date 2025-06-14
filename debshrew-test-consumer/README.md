# Debshrew Test Consumer

A simple Kafka consumer for testing Debshrew CDC messages. This tool consumes messages from a Kafka topic and logs them to stdout, making it easy to monitor and debug the CDC messages produced by Debshrew.

## Usage

### Using the Configuration File

1. Edit the `config.json` file to match your Kafka setup:

```json
{
  "bootstrap_servers": "localhost:9092",
  "topic": "cdc-events",
  "group_id": "debshrew-test-consumer",
  "auto_offset_reset": "earliest",
  "session_timeout_ms": 30000
}
```

2. Run the consumer using the provided script:

```bash
./run-consumer.sh
```

Or manually:

```bash
cargo run --bin debshrew-test-consumer -- run --sink-config ./debshrew-test-consumer/config.json --pretty --log-level info
```

### Using Command Line Arguments

You can also specify Kafka connection details directly via command line arguments:

```bash
cargo run --bin debshrew-test-consumer -- run \
  --bootstrap-servers localhost:9092 \
  --topic cdc-events \
  --group-id my-consumer-group \
  --auto-offset-reset earliest \
  --pretty \
  --log-level info
```

## Command Line Options

- `--sink-config <PATH>`: Path to the sink configuration file
- `--bootstrap-servers <SERVERS>`: Kafka bootstrap servers (e.g., "localhost:9092")
- `--topic <TOPIC>`: Kafka topic to consume from
- `--group-id <GROUP_ID>`: Consumer group ID
- `--auto-offset-reset <RESET>`: Auto offset reset strategy ("earliest" or "latest")
- `--pretty`: Pretty print JSON messages
- `--log-level <LEVEL>`: Log level (default: "info")

## Output Format

When a CDC message is received, it will be logged with the following format:

```
[INFO] Received CDC message: <OPERATION> <TABLE> <KEY> (block: <BLOCK_HEIGHT>)
```

If the `--pretty` flag is specified, the full JSON message will also be printed:

```json
{
  "header": {
    "source": "example-transform",
    "timestamp": "2025-06-13T17:26:57.123Z",
    "block_height": 123,
    "block_hash": "000000000000000000024bead8df69990852c202db0e0097c1a12ea637d7e96d",
    "transaction_id": null
  },
  "payload": {
    "operation": "CREATE",
    "table": "example_table",
    "key": "example_key",
    "before": null,
    "after": {
      "field1": "value1",
      "field2": 42
    }
  }
}
```

## Using with Debshrew

1. Start Debshrew with a Kafka sink:

```bash
./run.sh
```

2. In a separate terminal, start the test consumer:

```bash
./debshrew-test-consumer/run-consumer.sh
```

This will allow you to see the CDC messages being produced by Debshrew in real-time.