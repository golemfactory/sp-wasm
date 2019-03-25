use sp_wasm_engine::prelude::*;

fn main() {
    let start = Engine::new()
        .unwrap()
        .evaluate_script("golem_randEmu()")
        .unwrap()
        .to_number();
    let expected = 0.7641367265279992;

    assert_eq!(expected, start);
}
