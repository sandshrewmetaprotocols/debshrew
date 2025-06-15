#!/bin/bash

# Debshrew Test Consumer Runner
# This script helps you easily run the debshrew-test-consumer with proper configuration

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default configuration based on your docker-compose.yaml
DEFAULT_BOOTSTRAP_SERVERS="localhost:9092"
DEFAULT_TOPIC="cdc-events"
DEFAULT_GROUP_ID="debshrew-test-consumer"
DEFAULT_LOG_LEVEL="info"

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

# Function to check if Kafka is running
check_kafka() {
    print_status "Checking Kafka availability at $1..."
    if nc -z localhost 9092 2>/dev/null; then
        print_success "Kafka is available at $1"
        return 0
    else
        print_warning "Kafka doesn't seem to be running at $1"
        echo "To start Kafka using docker-compose, run:"
        echo "  docker-compose up -d kafka"
        echo ""
        echo "Continuing anyway (the consumer will wait for Kafka)..."
        return 1
    fi
}

# Function to show usage
show_usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  -s, --bootstrap-servers SERVERS  Kafka bootstrap servers (default: $DEFAULT_BOOTSTRAP_SERVERS)"
    echo "  -t, --topic TOPIC               Kafka topic to consume from (default: $DEFAULT_TOPIC)"
    echo "  -g, --group-id GROUP_ID         Consumer group ID (default: $DEFAULT_GROUP_ID)"
    echo "  -l, --log-level LEVEL           Log level: trace, debug, info, warn, error (default: $DEFAULT_LOG_LEVEL)"
    echo "  -p, --pretty                    Pretty print JSON messages"
    echo "  -c, --config FILE               Use configuration file instead of command line args"
    echo "  -r, --release                   Use release binary instead of cargo run"
    echo "  -h, --help                      Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0                              # Use default settings"
    echo "  $0 --pretty --log-level debug   # Pretty print with debug logging"
    echo "  $0 --config ./debshrew-test-consumer/config.json  # Use config file"
    echo "  $0 --topic my-topic --group-id my-group  # Custom topic and group"
    echo ""
    echo "Your current docker-compose.yaml setup:"
    echo "  Bootstrap servers: $DEFAULT_BOOTSTRAP_SERVERS"
    echo "  Suggested topic: $DEFAULT_TOPIC"
    echo "  Suggested group ID: $DEFAULT_GROUP_ID"
}

# Parse command line arguments
BOOTSTRAP_SERVERS="$DEFAULT_BOOTSTRAP_SERVERS"
TOPIC="$DEFAULT_TOPIC"
GROUP_ID="$DEFAULT_GROUP_ID"
LOG_LEVEL="$DEFAULT_LOG_LEVEL"
PRETTY=""
CONFIG_FILE=""
USE_RELEASE=""

while [[ $# -gt 0 ]]; do
    case $1 in
        -s|--bootstrap-servers)
            BOOTSTRAP_SERVERS="$2"
            shift 2
            ;;
        -t|--topic)
            TOPIC="$2"
            shift 2
            ;;
        -g|--group-id)
            GROUP_ID="$2"
            shift 2
            ;;
        -l|--log-level)
            LOG_LEVEL="$2"
            shift 2
            ;;
        -p|--pretty)
            PRETTY="--pretty"
            shift
            ;;
        -c|--config)
            CONFIG_FILE="$2"
            shift 2
            ;;
        -r|--release)
            USE_RELEASE="1"
            shift
            ;;
        -h|--help)
            show_usage
            exit 0
            ;;
        *)
            print_error "Unknown option: $1"
            show_usage
            exit 1
            ;;
    esac
done

# Print configuration
echo "========================================"
echo "  Debshrew Test Consumer"
echo "========================================"
echo ""

if [[ -n "$CONFIG_FILE" ]]; then
    print_status "Using configuration file: $CONFIG_FILE"
    if [[ ! -f "$CONFIG_FILE" ]]; then
        print_error "Configuration file not found: $CONFIG_FILE"
        exit 1
    fi
else
    print_status "Configuration:"
    echo "  Bootstrap servers: $BOOTSTRAP_SERVERS"
    echo "  Topic: $TOPIC"
    echo "  Group ID: $GROUP_ID"
    echo "  Log level: $LOG_LEVEL"
    echo "  Pretty print: $([ -n "$PRETTY" ] && echo "enabled" || echo "disabled")"
fi

echo ""

# Check Kafka availability
check_kafka "$BOOTSTRAP_SERVERS"
echo ""

# Build the consumer if not using release
if [[ -z "$USE_RELEASE" ]]; then
    print_status "Building debshrew-test-consumer..."
    if cargo build --bin debshrew-test-consumer; then
        print_success "Build completed successfully"
    else
        print_error "Build failed"
        exit 1
    fi
    echo ""
fi

# Prepare the command
if [[ -n "$CONFIG_FILE" ]]; then
    # Use configuration file
    if [[ -n "$USE_RELEASE" ]]; then
        CMD="./target/release/debshrew-test-consumer run --sink-config $CONFIG_FILE --log-level $LOG_LEVEL $PRETTY"
    else
        CMD="cargo run --bin debshrew-test-consumer -- run --sink-config $CONFIG_FILE --log-level $LOG_LEVEL $PRETTY"
    fi
else
    # Use command line arguments
    if [[ -n "$USE_RELEASE" ]]; then
        CMD="./target/release/debshrew-test-consumer run --bootstrap-servers $BOOTSTRAP_SERVERS --topic $TOPIC --group-id $GROUP_ID --auto-offset-reset earliest --log-level $LOG_LEVEL $PRETTY"
    else
        CMD="cargo run --bin debshrew-test-consumer -- run --bootstrap-servers $BOOTSTRAP_SERVERS --topic $TOPIC --group-id $GROUP_ID --auto-offset-reset earliest --log-level $LOG_LEVEL $PRETTY"
    fi
fi

# Run the consumer
print_status "Starting Debshrew test consumer..."
print_status "Command: $CMD"
echo ""
print_status "Press Ctrl+C to stop the consumer"
echo ""

# Execute the command
exec $CMD