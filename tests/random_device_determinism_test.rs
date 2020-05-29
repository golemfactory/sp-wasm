use sp_wasm_engine::prelude::*;

#[test]
fn random_device_determinism() {
    let engine = Engine::new().unwrap();
    let start = Runtime::new(&engine)
        .unwrap()
        .evaluate_script("golem_randEmu()")
        .unwrap()
        .to_number();
    let expected = 0.7641367265279992;

    assert_eq!(expected, start);
}
