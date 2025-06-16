#[cfg(feature = "test-utils")]
mod tests {
    use debshrew_runtime::println;
    use debshrew_runtime::test_utils::TestRunner;
    use debshrew_runtime::transform::DebTransform;
    use debshrew_runtime::error::Result;
    use debshrew_support::CdcMessage;
    use serde::{Serialize, Deserialize};
    use std::fmt::Debug;

    #[derive(Default, Clone, Serialize, Deserialize, Debug)]
    struct PrintlnTestTransform {
        counter: u32,
    }

    impl DebTransform for PrintlnTestTransform {
        fn process_block(&mut self) -> Result<Vec<CdcMessage>> {
            self.counter += 1;
            
            // Test the println! macro
            println!("Processing block: counter = {}", self.counter);
            
            Ok(vec![])
        }

        fn rollback(&mut self) -> Result<Vec<CdcMessage>> {
            self.counter -= 1;
            
            // Test the println! macro
            println!("Rolling back: counter = {}", self.counter);
            
            Ok(vec![])
        }
    }

    #[test]
    fn test_println_macro() {
        let runner = TestRunner::new()
            .with_height(123)
            .with_hash(vec![1, 2, 3, 4]);
        
        let mut transform = PrintlnTestTransform::default();
        
        // This should print "Processing block: counter = 1" to stdout
        let result = runner.run_transform(&mut transform);
        assert!(result.is_ok());
        
        // This should print "Rolling back: counter = 0" to stdout
        let result = runner.run_rollback(&mut transform);
        assert!(result.is_ok());
    }
}