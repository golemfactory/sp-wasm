use sp_wasm_engine::prelude::*;

#[test]
fn random_device_emulation() {
    let engine = Engine::new().unwrap();
    let runtime = Runtime::new(&engine).unwrap();
    let v1 = runtime
        .evaluate_script("golem_randEmu()")
        .unwrap()
        .to_number();
    let v2 = runtime
        .evaluate_script("golem_randEmu()")
        .unwrap()
        .to_number();
    let v3 = runtime
        .evaluate_script("golem_randEmu()")
        .unwrap()
        .to_number();

    assert_ne!(v1, v2);
    assert_ne!(v2, v3);
}
