use std::path;
use std::process;

fn main() {
    let tests: Vec<String> = path::Path::new("sp-wasm-tests/src/bin")
        .read_dir()
        .expect("failed to extract tests from /src/bin")
        .filter_map(|f| {
            if let Ok(f) = f {
                if f.path().is_file() {
                    let f = path::PathBuf::from(
                        f.path()
                            .file_name()
                            .expect("failed to extract testcase filename"),
                    );
                    let test_name = f
                        .file_stem()
                        .expect("failed to extract testcase filestem")
                        .to_string_lossy()
                        .into_owned();
                    return Some(test_name);
                }
            }
            None
        })
        .collect();

    let mut success = 0;
    let mut fail = 0;
    let mut error = 0;
    let total = tests.len();
    println!("running {} integration tests\n", total);

    for test in tests {
        match process::Command::new(format!("./target/debug/{}", test)).output() {
            Ok(output) => {
                if output.status.success() {
                    success += 1;
                    println!("test {} ... ok", test);
                } else {
                    fail += 1;
                    eprintln!(
                        "test {} ... failed\n{}",
                        test,
                        String::from_utf8(output.stderr).unwrap()
                    );
                }
            }
            Err(err) => {
                error += 1;
                eprintln!("test {} ... error\n{}", test, err);
            }
        }
    }

    let (result, status_code) = if success == total {
        ("ok", 0)
    } else {
        ("failed", 1)
    };
    println!(
        "\nintegration test result: {}. {} passed; {} failed; {} errored",
        result, success, fail, error,
    );
    process::exit(status_code)
}
