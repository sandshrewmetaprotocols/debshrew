#!/bin/bash

# Setup Kafka topic and run debshrew-test-consumer
# This script creates the required Kafka topic if it doesn't exist and then runs the consumer

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
BOOTSTRAP_SERVERS="localhost:9092"
TOPIC="cdc-events"
GROUP_ID="debshrew-test-consumer"
LOG_LEVEL="info"
PRETTY="--pretty"

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if Kafka is running
print_status "Checking Kafka availability..."
if nc -z localhost 9092 2>/dev/null; then
    print_success "Kafka is available at $BOOTSTRAP_SERVERS"
else
    print_warning "Kafka doesn't seem to be running at $BOOTSTRAP_SERVERS"
    print_status "Starting Kafka using docker-compose..."
    docker-compose up -d kafka
    
    # Wait for Kafka to start
    print_status "Waiting for Kafka to start (this may take a moment)..."
    for i in {1..30}; do
        if nc -z localhost 9092 2>/dev/null; then
            print_success "Kafka is now available!"
            break
        fi
        echo -n "."
        sleep 1
        if [ $i -eq 30 ]; then
            print_error "Timed out waiting for Kafka to start"
            exit 1
        fi
    done
    echo ""
fi

# Create the topic if it doesn't exist
print_status "Checking if topic '$TOPIC' exists..."

# Use docker to run kafka-topics command
TOPIC_EXISTS=$(docker exec $(docker ps -q --filter "name=kafka") kafka-topics.sh --bootstrap-server $BOOTSTRAP_SERVERS --list 2>/dev/null | grep "^$TOPIC$" || echo "")

if [ -z "$TOPIC_EXISTS" ]; then
    print_status "Topic '$TOPIC' does not exist. Creating it..."
    
    # Create the topic
    if docker exec $(docker ps -q --filter "name=kafka") kafka-topics.sh --bootstrap-server $BOOTSTRAP_SERVERS --create --topic $TOPIC --partitions 1 --replication-factor 1; then
        print_success "Topic '$TOPIC' created successfully"
    else
        print_error "Failed to create topic '$TOPIC'"
        exit 1
    fi
else
    print_success "Topic '$TOPIC' already exists"
fi

echo ""
print_status "Starting Debshrew test consumer..."
print_status "Configuration:"
echo "  - Bootstrap servers: $BOOTSTRAP_SERVERS"
echo "  - Topic: $TOPIC"
echo "  - Group ID: $GROUP_ID"
echo "  - Pretty print: enabled"
echo "  - Log level: $LOG_LEVEL"
echo ""
print_status "Press Ctrl+C to stop the consumer"
echo "----------------------------------------"

# Run the consumer
cargo run --bin debshrew-test-consumer -- run \
  --bootstrap-servers $BOOTSTRAP_SERVERS \
  --topic $TOPIC \
  --group-id $GROUP_ID \
  --auto-offset-reset earliest \
  $PRETTY \
  --log-level $LOG_LEVEL