#!/bin/bash
./target/release/debshrew run --metashrew-url http://localhost:18888 --transform ./target/wasm32-unknown-unknown/release/minimal_transform.wasm --sink-type kafka --sink-config ./examples/sample-transform/config.json --start-height 0 --log-level trace
