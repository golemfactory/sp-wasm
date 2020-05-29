use sp_wasm_engine::prelude::*;

#[test]
fn date() {
    let engine = Engine::new().unwrap();
    let runtime = Runtime::new(&engine).unwrap();
    let v1 = runtime.evaluate_script("Date.now()").unwrap().to_number();
    assert_eq!(v1 as u64, 0);
    let v2 = runtime.evaluate_script("Date.now()").unwrap().to_number();
    assert_eq!(v1, v2);
}
