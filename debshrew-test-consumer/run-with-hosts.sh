#!/bin/bash

# This script runs the debshrew-test-consumer with a temporary hosts entry for kafka
# This avoids the need for sudo privileges to modify /etc/hosts

echo "Starting Debshrew test consumer with kafka -> localhost mapping..."

# Run the consumer with a custom hosts entry
HOSTALIASES=/tmp/debshrew_hosts cargo run --bin debshrew-test-consumer -- run --sink-config ./debshrew-test-consumer/config.json --pretty --log-level debug

# Note: This creates a temporary hosts file that maps 'kafka' to 'localhost'
# The HOSTALIASES environment variable tells the system to use this file for hostname resolution