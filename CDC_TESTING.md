# CDC Message Testing Guide

## Understanding the Issue

When running debshrew and the test consumer, you might encounter a situation where the consumer doesn't receive any CDC messages. This is likely due to one of the following issues:

1. **Metashrew Synchronization Issue**: The logs show a discrepancy between the reported metashrew height (203) and the actual block count (1). This suggests that metashrew is not properly synced or running.

2. **Transform Module Not Generating Messages**: The transform module might not be generating CDC messages correctly.

3. **Kafka Configuration**: The Kafka topic might not exist or there might be connectivity issues.

## Testing the CDC Message Flow

To isolate and test the CDC message flow without relying on metashrew, you can use the `test-cdc-flow.sh` script. This script:

1. Checks if Kafka is running and starts it if needed
2. Creates the `cdc-events` topic if it doesn't exist
3. Sends a test CDC message directly to Kafka
4. Runs the consumer to verify the message was received

### Running the Test

```bash
./test-cdc-flow.sh
```

If the consumer successfully receives and displays the test message, it confirms that:
- Kafka is running correctly
- The topic exists
- The consumer is properly configured to receive messages

This means the issue is likely with metashrew or the transform module, not with the Kafka setup or consumer.

## Debugging the Full Flow

If the test script works but you're still not seeing messages when running debshrew, try these steps:

1. **Check Metashrew**: Make sure metashrew is running and properly synced. The logs show it's reporting a height of 203 but an actual block count of only 1.

2. **Examine Transform Module**: Look at the sample transform module to ensure it's correctly generating CDC messages:
   ```bash
   cat examples/sample-transform/src/lib.rs
   ```

3. **Check Debshrew Logs**: Look for any errors or warnings in the debshrew logs:
   ```bash
   ./run.sh | grep -i error
   ./run.sh | grep -i warn
   ```

4. **Verify Sink Configuration**: Make sure the sink configuration is correct:
   ```bash
   cat examples/sample-transform/sink-config.json
   ```

## Running with Debug Logging

To get more detailed logs, run debshrew with trace logging:

```bash
./run.sh --log-level trace
```

And in another terminal, run the consumer with debug logging:

```bash
./setup-and-run-consumer.sh --log-level debug
```

## Alternative Testing Approach

If you want to test the full flow without metashrew, you can modify the sample transform module to always generate CDC messages regardless of the block data. This would allow you to verify that the transform module is correctly sending messages to Kafka.