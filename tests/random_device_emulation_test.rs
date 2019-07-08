use sp_wasm_engine::prelude::*;

#[test]
fn random_device_emulation() {
    let engine = Engine::new().unwrap();
    let v1 = engine
        .evaluate_script("golem_randEmu()")
        .unwrap()
        .to_number();
    let v2 = engine
        .evaluate_script("golem_randEmu()")
        .unwrap()
        .to_number();
    let v3 = engine
        .evaluate_script("golem_randEmu()")
        .unwrap()
        .to_number();

    assert_ne!(v1, v2);
    assert_ne!(v2, v3);
}
