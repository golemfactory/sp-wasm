# SpiderMonkey-based WebAssembly Sandbox
[![Build Status]][travis] [![Rustc 1.33]][rustc] [![License]][license]

[Build Status]: https://travis-ci.org/golemfactory/sp-wasm.svg?branch=master
[travis]: http://travis-ci.org/golemfactory/sp-wasm
[Rustc 1.33]: https://img.shields.io/badge/rustc-1.33+-lightgray.svg
[rustc]: https://blog.rust-lang.org/2019/02/28/Rust-1.33.0.html
[License]: https://img.shields.io/github/license/golemfactory/sp-wasm.svg 
[license]: https://www.gnu.org/licenses/gpl-3.0.en.html

A proof-of-concept implementation of standalone WebAssembly sandbox using embedded SpiderMonkey engine. For `v8` version, see [golemfactory/wasm](https://github.com/golemfactory/wasm).

## Building
The implementation depends on [servo/rust-mozjs](https://github.com/servo/rust-mozjs) which in turn depends on
[servo/mozjs](https://github.com/servo/mozjs). The latter is Mozilla's Servo's SpiderMonkey fork and low-level
Rust bindings, and as such, requires C/C++ compiler and Autoconf 2.13.
See [servo/mozjs/README.md](https://github.com/servo/mozjs) for detailed building instructions.

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
sp_wasm -I <input-dir> -O <output-dir> -j <wasm-js> -w <wasm> -o <output-file>... -- <args>...
```

where
* `-I` path to the input dir
* `-O` path to the output dir
* `-j` path to the Emscripten JS glue script
* `-w` path to the Emscripten WASM binary
* `-o` paths to expected output files
* `--` anything after this will be passed to the WASM binary as arguments

**NB when building your WASM binary, make sure you pass in BINARYEN_ASYNC_COMPILATION=0 flag to Emscripten compiler.**

By default, basic logging is enabled. If you would like to enable more comprehensive logging, export
the following variable

```
RUST_LOG=debug
```

## License
[GPL-3.0](LICENSE)
