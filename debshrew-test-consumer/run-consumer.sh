#!/bin/bash

# Check if Kafka is running
echo "Checking Kafka availability..."
if nc -z localhost 9092 2>/dev/null; then
    echo "Kafka is available at localhost:9092"
else
    echo "Warning: Kafka doesn't seem to be running at localhost:9092"
    echo "Make sure Kafka is running before starting the consumer"
    echo "Continuing anyway..."
fi

# Run the debshrew test consumer
echo "Starting Debshrew test consumer..."
cargo run --bin debshrew-test-consumer -- run --sink-config ./debshrew-test-consumer/config.json --pretty --log-level debug