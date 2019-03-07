# SpiderMonkey-based WebAssembly Sandbox
A proof-of-concept implementation of standalone WebAssembly sandbox using embedded SpiderMonkey engine. For `v8` version, see [golemfactory/wasm](https://github.com/golemfactory/wasm).

## Building
The implementation depends on [servo/rust-mozjs](https://github.com/servo/rust-mozjs) which in turn depends on [servo/mozjs](https://github.com/servo/mozjs). The latter is Mozilla's Servo's SpiderMonkey fork and low-level Rust bindings, and as such, requires C/C++ compiler and Autoconf 2.13. See [servo/mozjs/README.md](https://github.com/servo/mozjs) for detailed building instructions.

Additionally, you will need `nightly` toolchain installed. Assuming you're using `rustup` to manage your rust versions, run

```
$ rustup override set nightly
```

After following the aforementioned instructions, to build the sandbox, run

```
$ cargo build
```

If you would like to build with SpiderMonkey's debug symbols and extensive logging, run instead

```
$ cargo build --features "log-debug"
```

## Running

```
sp_wasm -I <input-dir> -O <output-dir> -j <wasm-js> -w <wasm> -o <output-file>...
```

where
* `-I` path to the input dir
* `-O` path to the output dir
* `-j` path to the Emscripten JS glue script
* `-w` path to the Emscripten WASM binary
* `-o` paths to expected output files

**NB when building your WASM binary, make sure you pass in BINARYEN_ASYNC_COMPILATION=0 flag to Emscripten compiler.**

## License
[GPL-3.0](LICENSE)