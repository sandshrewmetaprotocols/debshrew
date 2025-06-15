# Debshrew Test Consumer Setup

This document explains how to set up and run the Debshrew test consumer to monitor CDC messages produced by Debshrew.

## Prerequisites

- Docker and Docker Compose installed (for Kafka)
- Rust and Cargo installed
- The debshrew project built successfully

## Available Scripts

We've created several scripts to make it easier to run the test consumer:

### 1. `setup-and-run-consumer.sh`

This script:
- Checks if Kafka is running and starts it if needed
- Creates the `cdc-events` topic if it doesn't exist
- Runs the test consumer

```bash
./setup-and-run-consumer.sh
```

### 2. `run-test-consumer.sh`

A more flexible script with command-line options:

```bash
./run-test-consumer.sh [OPTIONS]
```

Options:
- `-s, --bootstrap-servers SERVERS`: Kafka bootstrap servers (default: localhost:9092)
- `-t, --topic TOPIC`: Kafka topic to consume from (default: cdc-events)
- `-g, --group-id GROUP_ID`: Consumer group ID (default: debshrew-test-consumer)
- `-l, --log-level LEVEL`: Log level (default: info)
- `-p, --pretty`: Pretty print JSON messages
- `-c, --config FILE`: Use configuration file instead of command line args
- `-r, --release`: Use release binary instead of cargo run
- `-h, --help`: Show help message

### 3. `quick-consumer.sh`

A simple script for quick testing:

```bash
./quick-consumer.sh
```

### 4. `run-debshrew-with-consumer.sh`

This script runs both debshrew and the test consumer together:
- Starts Kafka if needed
- Creates the topic if needed
- Builds both debshrew and the test consumer
- Runs debshrew in one terminal and the consumer in another (if possible)

```bash
./run-debshrew-with-consumer.sh
```

## Manual Setup

If you prefer to set things up manually:

### 1. Start Kafka

```bash
docker-compose up -d kafka
```

### 2. Create the topic

```bash
docker exec $(docker ps -q --filter "name=kafka") kafka-topics.sh \
  --bootstrap-server localhost:9092 \
  --create --topic cdc-events \
  --partitions 1 --replication-factor 1
```

### 3. Run the consumer

```bash
cargo run --bin debshrew-test-consumer -- run \
  --bootstrap-servers localhost:9092 \
  --topic cdc-events \
  --group-id debshrew-test-consumer \
  --auto-offset-reset earliest \
  --pretty \
  --log-level info
```

### 4. Run debshrew (in another terminal)

```bash
cargo run --bin debshrew
```

## Troubleshooting

### Unknown Topic or Partition Error

If you see this error:
```
Error while receiving message: Message consumption error: UnknownTopicOrPartition (Broker: Unknown topic or partition)
```

It means the Kafka topic doesn't exist. Use the `setup-and-run-consumer.sh` script which will create the topic automatically.

### Kafka Connection Issues

If the consumer can't connect to Kafka, make sure Kafka is running:

```bash
docker ps | grep kafka
```

If it's not running, start it:

```bash
docker-compose up -d kafka
```

### No Messages Being Received

If the consumer is running but not receiving any messages, check that:

1. Debshrew is running and configured to send messages to the same Kafka topic
2. The transform module is correctly producing CDC messages
3. The Kafka topic exists and is correctly configured