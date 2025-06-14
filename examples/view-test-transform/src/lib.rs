use debshrew_runtime::{self, DebTransform};
use debshrew_runtime::{CdcMessage, CdcHeader, CdcOperation, CdcPayload};

#[derive(Default, Clone, Debug)]
pub struct ViewTestTransform {
    // State fields (none needed for this simple transform)
}

impl DebTransform for ViewTestTransform {
    fn process_block(&mut self) -> debshrew_runtime::Result<()> {
        // Get current block info
        let height = debshrew_runtime::get_height();
        let hash = debshrew_runtime::get_block_hash();
        
        debshrew_runtime::println!("ViewTestTransform: Processing block {} with hash {}", height, hex::encode(&hash));
        
        // Test the view function with a simple string parameter
        debshrew_runtime::println!("Testing view function with simple string parameter");
        
        // First, try a simple test with a string parameter
        let test_param = "test_param".as_bytes().to_vec();
        match debshrew_runtime::view("test".to_string(), test_param) {
            Ok(result) => {
                debshrew_runtime::println!("View function test succeeded! Result: {:?}", result);
                if let Ok(result_str) = String::from_utf8(result.clone()) {
                    debshrew_runtime::println!("Result as string: {}", result_str);
                }
            },
            Err(e) => {
                debshrew_runtime::eprintln!("View function test failed: {:?}", e);
            }
        }
        
        // Now try to call the getblock view with a simple parameter
        debshrew_runtime::println!("Testing getblock view with simple parameter");
        let getblock_param = format!("{{\"height\": {}}}", height).as_bytes().to_vec();
        match debshrew_runtime::view("getblock".to_string(), getblock_param) {
            Ok(result) => {
                debshrew_runtime::println!("getblock view succeeded! Result length: {} bytes", result.len());
            },
            Err(e) => {
                debshrew_runtime::eprintln!("getblock view failed: {:?}", e);
            }
        }
        
        // Create a simple CDC message
        let message = CdcMessage {
            header: CdcHeader {
                source: "view_test_transform".to_string(),
                timestamp: 1623456789000, // Fixed timestamp in milliseconds
                block_height: height,
                block_hash: hex::encode(&hash),
                transaction_id: None,
            },
            payload: CdcPayload {
                operation: CdcOperation::Create,
                table: "blocks".to_string(),
                key: height.to_string(),
                before: None,
                after: Some(serde_json::json!({
                    "height": height,
                    "hash": hex::encode(&hash),
                    "timestamp": 1623456789 // Fixed timestamp in seconds
                })),
            },
        };
        
        // Push CDC message to Kafka
        debshrew_runtime::println!("ViewTestTransform: Pushing CDC message to Kafka...");
        self.push_message(message)?;
        
        debshrew_runtime::println!("ViewTestTransform: Block processing complete");
        
        Ok(())
    }
}

// Register the transform
debshrew_runtime::declare_transform!(ViewTestTransform);