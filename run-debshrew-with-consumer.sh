#!/bin/bash

# Run both debshrew and the test consumer in separate terminals
# This script starts Kafka, creates the topic, runs debshrew, and runs the consumer

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

# Check if gnome-terminal or xterm is available
if command -v gnome-terminal &> /dev/null; then
    TERMINAL="gnome-terminal --"
elif command -v xterm &> /dev/null; then
    TERMINAL="xterm -e"
else
    print_warning "Neither gnome-terminal nor xterm found. Will run in sequence instead of parallel terminals."
    TERMINAL=""
fi

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

echo ""
print_status "Building debshrew and test consumer..."
cargo build --release -p debshrew -p debshrew-test-consumer

echo ""
print_status "Starting debshrew and test consumer..."

if [ -n "$TERMINAL" ]; then
    # Start debshrew in a new terminal
    print_status "Starting debshrew in a new terminal..."
    $TERMINAL bash -c "echo 'Starting debshrew...'; ./target/release/debshrew; echo 'Press Enter to close this terminal'; read"
    
    # Start the consumer in this terminal
    print_status "Starting test consumer in this terminal..."
    ./target/release/debshrew-test-consumer run \
      --bootstrap-servers localhost:9092 \
      --topic cdc-events \
      --group-id debshrew-test-consumer \
      --auto-offset-reset earliest \
      --pretty \
      --log-level info
else
    # No terminal available, run sequentially
    print_warning "Running debshrew in the background and consumer in the foreground"
    print_status "Starting debshrew in the background..."
    ./target/release/debshrew &
    DEBSHREW_PID=$!
    
    # Give debshrew a moment to start
    sleep 2
    
    print_status "Starting test consumer in the foreground..."
    print_status "Press Ctrl+C to stop both debshrew and the consumer"
    
    # Set up trap to kill debshrew when the script exits
    trap "kill $DEBSHREW_PID 2>/dev/null || true; echo ''; print_status 'Stopped debshrew and consumer'" EXIT
    
    # Run the consumer
    ./target/release/debshrew-test-consumer run \
      --bootstrap-servers localhost:9092 \
      --topic cdc-events \
      --group-id debshrew-test-consumer \
      --auto-offset-reset earliest \
      --pretty \
      --log-level info
fi