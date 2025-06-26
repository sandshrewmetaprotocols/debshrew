# Debshrew Test Suite

This directory contains the comprehensive test suite for debshrew, following the same patterns as metashrew's test architecture for maximum coverage and maintainability.

## Architecture

The test suite is built around generic traits and in-memory adapters that allow for fast, comprehensive testing without requiring external services:

### Core Components

- **`traits.rs`** - Generic traits for metashrew client backends (`MetashrewClientLike`, `BlockProviderLike`, `ViewProviderLike`)
- **`adapters.rs`** - In-memory implementations of the traits for testing (`MemoryMetashrewAdapter`)
- **`debshrew-minimal/`** - Minimal transform WASM module for end-to-end testing
- **`tests/`** - Comprehensive test modules

### Test Modules

1. **`mod.rs`** - Test configuration and utilities
2. **`block_builder.rs`** - Utilities for creating test Bitcoin blocks and chains
3. **`comprehensive_e2e_test.rs`** - End-to-end functionality tests
4. **`reorg_focused_test.rs`** - Chain reorganization and CDC rollback tests
5. **`integration_tests.rs`** - Complete workflow integration tests

## Key Features

### Generic Design
- Uses traits for dependency injection
- Allows testing with in-memory mocks or real metashrew services
- Follows the same patterns as metashrew's memshrew adapter

### Comprehensive Coverage
- Block processing and CDC generation
- Chain reorganizations and rollbacks
- Historical queries and state consistency
- Error handling and edge cases

### Fast Execution
- In-memory adapters for speed
- No external dependencies required
- Parallel test execution safe

### Real WASM Testing
- Uses actual debshrew-minimal WASM module
- Tests complete transform lifecycle
- Validates CDC message generation

## Running Tests

### Basic Tests
```bash
# Run all tests
cargo test

# Run specific test module
cargo test comprehensive_e2e_test
cargo test reorg_focused_test
cargo test integration_tests
```

### With WASM Building
```bash
# Force WASM rebuild
DEBSHREW_BUILD_WASM=1 cargo test

# Run tests with logging
RUST_LOG=debug cargo test -- --nocapture
```

### Test Categories

#### Unit Tests
- Individual component functionality
- Trait implementations
- Utility functions

#### Integration Tests
- Complete workflow testing
- Multi-component interactions
- State management

#### End-to-End Tests
- Full debshrew pipeline
- WASM transform execution
- CDC message validation

## Test Data

### Block Builder
The `block_builder` module provides utilities for creating realistic test scenarios:

```rust
// Create a simple chain
let chain = ChainBuilder::new().add_blocks(5).blocks();

// Create reorg scenario
let (original, reorg) = create_reorg_scenario(2, 3, 4);

// Create custom blocks
let block = BlockBuilder::new()
    .height(10)
    .content(b"custom_data")
    .build();
```

### Memory Adapter
The `MemoryMetashrewAdapter` provides a complete in-memory metashrew implementation:

```rust
// Create adapter
let adapter = MemoryMetashrewAdapter::new();

// Set up test data
adapter.set_height(100);
adapter.set_block_hash(100, vec![0xaa, 0xbb]);
adapter.set_view_result("test_view", &[1, 2], Some(100), vec![3, 4]);

// Use like real metashrew client
let height = adapter.get_height().await?;
let result = adapter.call_view("test_view", &[1, 2], Some(100)).await?;
```

## Debshrew-Minimal Transform

The `debshrew-minimal` crate provides a simple transform for testing:

### Features
- Tracks block processing via blocktracker view
- Generates CDC messages for each block
- Supports rollback with inverse CDC messages
- Validates metashrew view integration

### CDC Messages
Each processed block generates a CDC message:
```json
{
  "header": {
    "source": "debshrew-minimal",
    "block_height": 5,
    "block_hash": "aa",
    "timestamp": 1234567890
  },
  "payload": {
    "operation": "Create",
    "table": "blocks",
    "key": "5",
    "after": {
      "height": 5,
      "hash_byte": 170,
      "tracker_length": 6
    }
  }
}
```

## Test Patterns

### Reorg Testing
```rust
// Process initial chain
for (height, block) in initial_blocks.iter().enumerate() {
    processor.process_block(height as u32, block)?;
}

// Simulate rollback
let rollback_messages = processor.process_rollback(target_height)?;

// Process reorg blocks
for (height, block) in reorg_blocks.iter().enumerate() {
    processor.process_block(fork_height + height as u32, block)?;
}

// Verify CDC consistency
assert_eq!(rollback_messages.len(), expected_rollback_count);
```

### View Function Testing
```rust
// Set up metashrew views
TestUtils::setup_metashrew_minimal_views(&adapter, &blocks);

// Test blocktracker at different heights
for height in 0..blocks.len() {
    let bt = adapter.call_view("blocktracker", &[], Some(height)).await?;
    assert_eq!(bt.len(), height + 1);
}
```

## Extending Tests

### Adding New Test Cases
1. Create test in appropriate module
2. Use existing utilities (`TestUtils`, `BlockBuilder`)
3. Follow existing patterns for setup and verification

### Adding New Adapters
1. Implement the core traits (`MetashrewClientLike`, etc.)
2. Add to `adapters.rs` or create new module
3. Update test configuration as needed

### Adding New Transforms
1. Create new crate following `debshrew-minimal` pattern
2. Implement `DebTransform` trait
3. Add to workspace and test configuration

## Performance

The test suite is optimized for speed:
- In-memory operations only
- Minimal WASM module
- Parallel execution safe
- No network dependencies

Typical execution times:
- Unit tests: < 1s
- Integration tests: < 5s
- Full suite: < 10s

## Debugging

### Common Issues
1. **WASM not found** - Run `DEBSHREW_BUILD_WASM=1 cargo test`
2. **View setup errors** - Check `TestUtils::setup_metashrew_minimal_views`
3. **CDC validation failures** - Use `TestUtils::compare_cdc_messages_ignore_timestamp`

### Logging
```bash
# Enable debug logging
RUST_LOG=debug cargo test -- --nocapture

# Test-specific logging
RUST_LOG=debshrew::tests=trace cargo test comprehensive_e2e_test -- --nocapture
```

This test architecture provides comprehensive coverage while maintaining fast execution and easy extensibility, following the proven patterns from metashrew's test suite.