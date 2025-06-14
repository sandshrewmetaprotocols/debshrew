#!/bin/bash

# Default transform module
TRANSFORM_MODULE="view_test_transform"

# Check if an argument was provided
if [ $# -gt 0 ]; then
    # If the argument is a directory, extract the module name
    if [ -d "$1" ]; then
        TRANSFORM_MODULE=$(basename "$1")
    else
        # Otherwise, use the argument as the module name
        TRANSFORM_MODULE="$1"
    fi
fi

# Build the transform module
echo "Building transform module: $TRANSFORM_MODULE"
cargo build --target wasm32-unknown-unknown --release --package $TRANSFORM_MODULE

# Convert hyphens to underscores for the WASM filename
WASM_FILENAME=$(echo ${TRANSFORM_MODULE} | tr '-' '_')

# Run debshrew with the specified transform module
./target/release/debshrew run \
    --metashrew-url http://localhost:18888 \
    --transform ./target/wasm32-unknown-unknown/release/${WASM_FILENAME}.wasm \
    --sink-type kafka \
    --sink-config ./examples/sample-transform/sink-config.json \
    --start-height 0 \
    --log-level trace
