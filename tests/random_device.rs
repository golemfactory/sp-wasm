mod common;

use sp_wasm::prelude::*;

#[test]
fn determinism() {
    let start = common::sandbox().get()
        .evaluate_script("golem_randEmu()")
        .unwrap()
        .to_number();
    let expected = 0.7641367265279992;

    assert_eq!(expected, start);
}

#[test]
fn emulation() {
    let engine = common::sandbox().get();
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
