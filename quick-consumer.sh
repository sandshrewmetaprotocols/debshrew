#!/bin/bash

# Quick and simple debshrew-test-consumer runner
# Uses the configuration from your docker-compose.yaml setup

echo "Starting Debshrew Test Consumer..."
echo "Configuration:"
echo "  - Bootstrap servers: localhost:9092"
echo "  - Topic: cdc-events"
echo "  - Group ID: debshrew-test-consumer"
echo "  - Pretty print: enabled"
echo "  - Log level: info"
echo ""

# Check if Kafka is running
if nc -z localhost 9092 2>/dev/null; then
    echo "✅ Kafka is available at localhost:9092"
else
    echo "⚠️  Kafka doesn't seem to be running. Start it with: docker-compose up -d kafka"
fi

echo ""
echo "Press Ctrl+C to stop the consumer"
echo "----------------------------------------"

# Run the consumer with your configuration
cargo run --bin debshrew-test-consumer -- run \
  --bootstrap-servers localhost:9092 \
  --topic cdc-events \
  --group-id debshrew-test-consumer \
  --auto-offset-reset earliest \
  --pretty \
  --log-level info