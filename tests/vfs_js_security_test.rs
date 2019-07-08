use sp_wasm_engine::error::Error;
use sp_wasm_engine::prelude::*;
use sp_wasm_engine::sandbox::engine::error::Error as EngineError;
use std::path;

#[test]
fn vfs_js_security() {
    let engine = Engine::new().unwrap();
    let result = engine.evaluate_script("writeFile('/tmp/test.txt', new Uint8Array(2))");

    assert!(result.is_err());
    assert!(!path::Path::new("/tmp/test.txt").is_file());

    match result {
        Err(Error::Engine(ref err)) => match err {
            EngineError::SMJS(ref err) => assert_eq!(
                err.message,
                "failed to write file '/tmp/test.txt' with error: file 'tmp' not found"
            ),
            _ => panic!("wrong error received"),
        },
        _ => panic!("wrong error received"),
    }
}
