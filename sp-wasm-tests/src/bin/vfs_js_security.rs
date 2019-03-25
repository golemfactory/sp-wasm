use sp_wasm_lib::prelude::*;

use std::path;

fn main() {
    let engine = Engine::new().unwrap();
    let result = engine.evaluate_script("writeFile('/tmp/test.txt', new Uint8Array(2))");

    assert!(result.is_err());
    assert!(!path::Path::new("/tmp/test.txt").is_file());

    if let Err(err) = result {
        assert_eq!(
            &err.message,
            "failed to write file with error: File not found"
        );
    }
}
