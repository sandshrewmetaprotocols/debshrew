#!/bin/bash

# Test CDC message flow by directly sending messages to Kafka
# This script bypasses the need for metashrew to be running

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

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
    print_success "Kafka is available at localhost:9092"
else
    print_warning "Kafka doesn't seem to be running"
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
print_status "Checking if topic 'cdc-events' exists..."

# Use docker to run kafka-topics command
TOPIC_EXISTS=$(docker exec $(docker ps -q --filter "name=kafka") kafka-topics.sh --bootstrap-server localhost:9092 --list 2>/dev/null | grep "^cdc-events$" || echo "")

if [ -z "$TOPIC_EXISTS" ]; then
    print_status "Topic 'cdc-events' does not exist. Creating it..."
    
    # Create the topic
    if docker exec $(docker ps -q --filter "name=kafka") kafka-topics.sh --bootstrap-server localhost:9092 --create --topic cdc-events --partitions 1 --replication-factor 1; then
        print_success "Topic 'cdc-events' created successfully"
    else
        print_error "Failed to create topic 'cdc-events'"
        exit 1
    fi
else
    print_success "Topic 'cdc-events' already exists"
fi

# Create a temporary file for the CDC message
CDC_FILE=$(mktemp)
print_status "Creating test CDC message in $CDC_FILE"

# Get current timestamp in milliseconds
TIMESTAMP=$(date +%s000)

# Create a sample CDC message
cat > $CDC_FILE << EOF
{
  "header": {
    "source": "test-script",
    "timestamp": $TIMESTAMP,
    "block_height": 123,
    "block_hash": "000000000000000000024bead8df69990852c202db0e0097c1a12ea637d7e96d",
    "transaction_id": null
  },
  "payload": {
    "operation": "CREATE",
    "table": "test_table",
    "key": "test_key_$(date +%s)",
    "before": null,
    "after": {
      "field1": "value1",
      "field2": 42,
      "timestamp": "$(date)"
    }
  }
}
EOF

print_status "Sending test CDC message to Kafka topic 'cdc-events'..."

# Send the message to Kafka
if docker exec -i $(docker ps -q --filter "name=kafka") kafka-console-producer.sh --bootstrap-server localhost:9092 --topic cdc-events < $CDC_FILE; then
    print_success "Message sent successfully"
else
    print_error "Failed to send message"
    rm $CDC_FILE
    exit 1
fi

# Clean up
rm $CDC_FILE

print_status "Starting the consumer to verify the message was received..."
print_status "Press Ctrl+C to stop the consumer"
echo "----------------------------------------"

# Run the consumer
./target/release/debshrew-test-consumer run \
  --bootstrap-servers localhost:9092 \
  --topic cdc-events \
  --group-id debshrew-test-consumer \
  --auto-offset-reset earliest \
  --pretty \
  --log-level info