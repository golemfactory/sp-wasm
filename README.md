# SpiderMonkey-based WebAssembly Sandbox
[![Build Status]][travis] [![Rustc 1.33]][rustc] [![License]][license]

[Build Status]: https://travis-ci.org/golemfactory/sp-wasm.svg?branch=master
[travis]: http://travis-ci.org/golemfactory/sp-wasm
[Rustc 1.33]: https://img.shields.io/badge/rustc-1.33+-lightgray.svg
[rustc]: https://blog.rust-lang.org/2019/02/28/Rust-1.33.0.html
[License]: https://img.shields.io/github/license/golemfactory/sp-wasm.svg 
[license]: https://www.gnu.org/licenses/gpl-3.0.en.html

A WebAssembly sandbox using standalone SpiderMonkey engine. For `v8` version,
see [golemfactory/v8-wasm](https://github.com/golemfactory/v8-wasm).

This WebAssembly sandbox is used in current development version of
Golem: [golem/apps/wasm](https://github.com/golemfactory/golem/tree/develop/apps/wasm).
If you would like to launch a Wasm task in Golem, see
[here](https://github.com/golemfactory/golem/wiki/Launching-Wasm-tasks-in-Golem).

- [SpiderMonkey-based WebAssembly Sandbox](#spidermonkey-based-webassembly-sandbox)
  - [Quick start guide](#quick-start-guide)
    - [1. Create simple C program](#1-create-simple-c-program)
    - [2. Create input and output dirs and files](#2-create-input-and-output-dirs-and-files)
    - [3. Compile to Wasm using Emscripten](#3-compile-to-wasm-using-emscripten)
    - [4. Run](#4-run)
  - [Build instructions](#build-instructions)
  - [CLI arguments explained](#cli-arguments-explained)
  - [Caveats](#caveats)
  - [Wasm store](#wasm-store)
  - [Contributing](#contributing)
  - [License](#license)

## Quick start guide
This guide assumes you have successfully built the `wasm-sandbox` binary; for build instructions, see section
[Build instructions](#build-instructions) below.

### 1. Create simple C program
Let us create a simple `hello world` style C program which will read
in some text from `in.txt` text file, read your name from the command
line, and save the resultant text in `out.txt`.

```c
#include <stdio.h>

int main(int argc, char** argv) {
  char* name = argc >= 2 ? argv[1] : "anonymous";
  size_t len = 0;
  char* line = NULL;
  ssize_t read;
  
  FILE* f_in = fopen("in.txt", "r");
  FILE* f_out = fopen("out.txt", "w");
  
  while ((read = getline(&line, &len, f_in)) != -1)
      fprintf(f_out, "%s\n", line);
  
  fprintf(f_out, "%s\n", name);
  
  fclose(f_out);
  fclose(f_in);
  
  return 0;
}
```

There is one important thing to notice here. The sandbox communicates
the results of computation by reading and writing to files. Thus, every
Wasm program is required to at the very least create an output file.
If your code does not include file manipulation in its main body,
then the Emscripten compiler, by default, will not initialise
JavaScript `FS` library, and will trip the sandbox.

### 2. Create input and output dirs and files
The sandbox will require us to specify input and output paths together
with output filenames to create, and any additional arguments (see
[CLI arguments explained](#cli-arguments-explained) section below
for detailed specification
of the required arguments). Suppose we have the following file
structure locally

```
  |-- simple.c
  |
  |-- in/
  |    |
  |    |-- in.txt
  |
  |-- out/
```

Paste the following text in the `in.txt` file

```
// in.txt
You are running Wasm!
```

### 3. Compile to Wasm using Emscripten
For this step, you will need the Emscripten SDK installed on your
system. For instructions on how to do it, see [here](https://emscripten.org/docs/getting_started/downloads.html).

```
$ emcc -o simple.js -s BINARYEN_ASYNC_COMPILATION=0 simple.c
```

Emscripten will then produce two files: `simple.js` and `simple.wasm`.
The produced JavaScript file acts as glue code and sets up all of
the rudimentary syscalls in JavaScript such as `MemFS` (in-memory
filesystem), etc., while the `simple.wasm` is our C program
cross-compiled to Wasm.

Note here the compiler flag `-s BINARYEN_ASYNC_COMPILATION=0`. By
default, the Emscripten compiler enables async IO lib when
cross-compiling to Wasm which we currently do not support.
Therefore, in order to alleviate the problem, make sure to always
cross-compile with `-s BINARYEN_ASYNC_COMPILATION=0` flag.

### 4. Run
After you have successfully run all of the above steps up to now, you should have the following file structure locally

```
  |-- simple.c
  |-- simple.js
  |-- simple.wasm
  |
  |-- in/
  |    |
  |    |-- in.txt
  |
  |-- out/
```

We can now run our Wasm binary inside the sandbox

```
wasm-sandbox -I in/ -O out/ -j simple.js -w simple.wasm \
             -o out.txt -- "<your_name>"
```

Here, `-I` maps the input dir with *all* its contents (files and
subdirs) directly to the root `/` in `MemFS`. The output files,
on the other hand, will be saved in `out/` local dir. The names of
the expected output files have to match those specified with `-o`
flags. Thus, in this case, our Wasm bin is expected to create an
output file `/out.txt` in `MemFS` which will then be saved in
`out/out.txt` locally.

After you execute Wasm bin in the sandbox, `out.txt` should be
created in `out/` dir

```
  |-- simple.c
  |-- simple.js
  |-- simple.wasm
  |
  |-- in/
  |    |
  |    |-- in.txt
  |
  |-- out/
  |    |
  |    |-- out.txt
```

with the contents similar to the following

```
// out.txt
You are running Wasm!
<your_name>
```

## Build instructions
The implementation depends on [servo/rust-mozjs](https://github.com/servo/rust-mozjs) which in turn depends on
[servo/mozjs](https://github.com/servo/mozjs). The latter is Mozilla's Servo's SpiderMonkey fork and low-level
Rust bindings, and as such, requires C/C++ compiler and Autoconf 2.13. See [servo/mozjs/README.md](https://github.com/servo/mozjs) for detailed building instructions.

After following the aforementioned instructions, to build the sandbox, run

```
$ cargo build
```

If you would like to build with SpiderMonkey's debug symbols and extensive logging, run instead

```
$ cargo build --features "debugmozjs"
```

## CLI arguments explained
```
wasm-sandbox -I <input-dir> -O <output-dir> -j <wasm-js> -w <wasm> -o <output-file>... -- <args>...
```

where
* `-I` path to the input dir
* `-O` path to the output dir
* `-j` path to the Emscripten JS glue script
* `-w` path to the Emscripten WASM binary
* `-o` paths to expected output files
* `--` anything after this will be passed to the WASM binary as arguments

By default, basic logging is enabled. If you would like to enable more comprehensive logging, export
the following variable

```
RUST_LOG=debug
```

## Caveats
* If you were following the [Quick start guide](#quick-start-guide)
  you already know that
  every Wasm bin needs to be cross-compiled by Emscripten with
  `-s BINARYEN_ASYNC_COMPILATION=0` flag in order to turn off the use
  of async IO which we currently don't support.
* Sometimes, if the binary you are cross-compiling is of substantial
  size, you might encounter a `asm2wasm` validation error stating
  that there is not enough memory assigned to Wasm. In this case,
  you can circumvent the problem by adding `-s TOTAL_MEMORY=value`
  flag. The value has to be an integer multiple of 1 Wasm memory page
  which is currently set at `65,536` bytes.
* When running your Wasm binary you encounter an `OOM` error at
  runtime, it usually means that the sandbox has run out-of-memory.
  To alleviate the problem, recompile your program with
  `-s ALLOW_MEMORY_GROWTH=1`.
* Emscripten, by default, doesn't support `/dev/(u)random` emulation
  targets different than either browser or `nodejs`. Therefore, we
  have added basic emulation of the random device that is *fully*
  deterministic. For details, see [#5](https://github.com/golemfactory/sp-wasm/pull/5).

## Wasm store
More examples of precompiled Wasm binaries can be found in [golemfactory/wasm-store](https://github.com/golemfactory/wasm-store) repo.

## Contributing
All contributions are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for pointers.

## License
Licensed under [GNU General Public License v3.0](LICENSE).
